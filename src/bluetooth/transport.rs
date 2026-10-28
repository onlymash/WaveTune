// WaveTune - Unofficial multi-brand wireless headset control center for Linux
// Copyright (C) 2026 onlymash <fiepi.dev@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    ffi::{CStr, c_char, c_int, c_uint, c_void},
    fs::File,
    io::ErrorKind,
    mem::size_of,
    os::fd::{FromRawFd, OwnedFd, RawFd},
    time::Duration,
};

use bluer::Address;

const SPP_SERVICE_UUID: u16 = 0x1101;
const RFCOMM_PROTOCOL_UUID: c_int = 0x0003;
const SDP_RETRY_IF_BUSY: c_uint = 0x01;
const SDP_ATTR_REQ_RANGE: c_int = 2;
const SDP_ATTR_SVCNAME_PRIMARY: u16 = 0x0100;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const SOCKET_POLL_TIMEOUT: Duration = Duration::from_millis(250);

#[repr(C)]
struct BluetoothAddress([u8; 6]);

#[repr(C)]
struct SdpList {
    next: *mut SdpList,
    data: *mut c_void,
}

#[repr(C)]
union SdpUuidValue {
    uuid16: u16,
    uuid32: u32,
    uuid128: [u8; 16],
}

#[repr(C)]
struct SdpUuid {
    kind: u8,
    value: SdpUuidValue,
}

#[link(name = "bluetooth")]
unsafe extern "C" {
    fn sdp_connect(
        source: *const BluetoothAddress,
        destination: *const BluetoothAddress,
        flags: c_uint,
    ) -> *mut c_void;
    fn sdp_close(session: *mut c_void);
    fn sdp_uuid16_create(uuid: *mut SdpUuid, value: u16) -> *mut SdpUuid;
    fn sdp_list_append(list: *mut SdpList, data: *mut c_void) -> *mut SdpList;
    fn sdp_list_free(list: *mut SdpList, free_data: Option<unsafe extern "C" fn(*mut c_void)>);
    fn sdp_service_search_attr_req(
        session: *mut c_void,
        search: *const SdpList,
        request_type: c_int,
        attributes: *const SdpList,
        response: *mut *mut SdpList,
    ) -> c_int;
    fn sdp_get_string_attr(
        record: *const c_void,
        attribute: u16,
        value: *mut c_char,
        length: usize,
    ) -> c_int;
    fn sdp_get_access_protos(record: *const c_void, protocols: *mut *mut SdpList) -> c_int;
    fn sdp_get_proto_port(protocols: *const SdpList, protocol: c_int) -> c_int;
    fn sdp_record_free(record: *mut c_void);
}

struct SdpSession(*mut c_void);

impl Drop for SdpSession {
    fn drop(&mut self) {
        unsafe { sdp_close(self.0) };
    }
}

unsafe extern "C" fn free_nested_sdp_list(data: *mut c_void) {
    unsafe { sdp_list_free(data.cast(), None) };
}

pub(super) fn open_rfcomm(address: Address) -> std::io::Result<File> {
    #[repr(C)]
    struct SockAddrRc {
        family: libc::sa_family_t,
        address: [u8; 6],
        channel: u8,
    }

    let raw_fd: RawFd = unsafe {
        libc::socket(
            libc::AF_BLUETOOTH,
            libc::SOCK_STREAM | libc::SOCK_CLOEXEC,
            3,
        )
    };
    if raw_fd == -1 {
        return Err(std::io::Error::last_os_error());
    }
    let owned_fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };
    set_socket_timeout(raw_fd, libc::SO_SNDTIMEO, CONNECT_TIMEOUT)?;
    let channel = resolve_elink_channel(address)?;
    let mut bluetooth_address = address.0;
    bluetooth_address.reverse();
    let socket_address = SockAddrRc {
        family: libc::AF_BLUETOOTH as libc::sa_family_t,
        address: bluetooth_address,
        channel,
    };
    let result = unsafe {
        libc::connect(
            raw_fd,
            &socket_address as *const SockAddrRc as *const libc::sockaddr,
            size_of::<SockAddrRc>() as libc::socklen_t,
        )
    };
    if result == -1 {
        return Err(std::io::Error::last_os_error());
    }

    for option in [libc::SO_RCVTIMEO, libc::SO_SNDTIMEO] {
        set_socket_timeout(raw_fd, option, SOCKET_POLL_TIMEOUT)?;
    }

    Ok(File::from(owned_fd))
}

const FALLBACK_RFCOMM_CHANNEL: u8 = 15;

fn is_control_service_name(name: &str) -> bool {
    let normalized = name.trim().to_ascii_lowercase();
    normalized == "elnk" || normalized == "enco link" || normalized == "encolink"
}

fn resolve_elink_channel(address: Address) -> std::io::Result<u8> {
    let local_address = BluetoothAddress([0; 6]);
    let mut remote_bytes = address.0;
    remote_bytes.reverse();
    let remote_address = BluetoothAddress(remote_bytes);
    let session = unsafe { sdp_connect(&local_address, &remote_address, SDP_RETRY_IF_BUSY) };
    if session.is_null() {
        return Ok(FALLBACK_RFCOMM_CHANNEL);
    }
    let session = SdpSession(session);

    let mut service_uuid = SdpUuid {
        kind: 0,
        value: SdpUuidValue { uuid16: 0 },
    };
    if unsafe { sdp_uuid16_create(&mut service_uuid, SPP_SERVICE_UUID) }.is_null() {
        return Ok(FALLBACK_RFCOMM_CHANNEL);
    }
    let search = unsafe {
        sdp_list_append(
            std::ptr::null_mut(),
            (&mut service_uuid as *mut SdpUuid).cast(),
        )
    };
    if search.is_null() {
        return Ok(FALLBACK_RFCOMM_CHANNEL);
    }
    let attribute_range: u32 = 0x0000_ffff;
    let attributes = unsafe {
        sdp_list_append(
            std::ptr::null_mut(),
            (&attribute_range as *const u32).cast_mut().cast(),
        )
    };
    if attributes.is_null() {
        unsafe { sdp_list_free(search, None) };
        return Ok(FALLBACK_RFCOMM_CHANNEL);
    }

    let mut records = std::ptr::null_mut();
    let result = unsafe {
        sdp_service_search_attr_req(
            session.0,
            search,
            SDP_ATTR_REQ_RANGE,
            attributes,
            &mut records,
        )
    };
    unsafe {
        sdp_list_free(search, None);
        sdp_list_free(attributes, None);
    }
    if result < 0 {
        return Ok(FALLBACK_RFCOMM_CHANNEL);
    }

    let mut selected_channel = None;
    let mut ambiguous_channel = false;
    let mut record_node = records;
    while !record_node.is_null() {
        let record = unsafe { (*record_node).data };
        let mut name = [0 as c_char; 257];
        let name_res = unsafe {
            sdp_get_string_attr(
                record,
                SDP_ATTR_SVCNAME_PRIMARY,
                name.as_mut_ptr(),
                name.len() - 1,
            )
        };
        let is_control = if name_res == 0 {
            let name_str = unsafe { CStr::from_ptr(name.as_ptr()) }.to_string_lossy();
            is_control_service_name(&name_str)
        } else {
            false
        };

        if is_control {
            let mut protocols = std::ptr::null_mut();
            if unsafe { sdp_get_access_protos(record, &mut protocols) } == 0 {
                let channel = unsafe { sdp_get_proto_port(protocols, RFCOMM_PROTOCOL_UUID) };
                unsafe { sdp_list_free(protocols, Some(free_nested_sdp_list)) };
                if (1..=30).contains(&channel) {
                    let channel = channel as u8;
                    if selected_channel.is_some_and(|existing| existing != channel) {
                        ambiguous_channel = true;
                    } else {
                        selected_channel = Some(channel);
                    }
                }
            }
        }
        unsafe { sdp_record_free(record) };
        record_node = unsafe { (*record_node).next };
    }
    unsafe { sdp_list_free(records, None) };
    if ambiguous_channel {
        return Err(std::io::Error::other(
            "SDP returned multiple conflicting ELNK RFCOMM channels",
        ));
    }
    Ok(selected_channel.unwrap_or(FALLBACK_RFCOMM_CHANNEL))
}

fn set_socket_timeout(fd: RawFd, option: libc::c_int, duration: Duration) -> std::io::Result<()> {
    let timeout = libc::timeval {
        tv_sec: duration.as_secs() as libc::time_t,
        tv_usec: duration.subsec_micros() as libc::suseconds_t,
    };
    let result = unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            option,
            &timeout as *const libc::timeval as *const libc::c_void,
            size_of::<libc::timeval>() as libc::socklen_t,
        )
    };
    if result == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

pub(super) fn is_socket_timeout(error: &std::io::Error) -> bool {
    matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
}
