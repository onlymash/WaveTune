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

use bluer::{Address, Uuid};

const SPP_SERVICE_UUID: u16 = 0x1101;
const RFCOMM_PROTOCOL_UUID: c_int = 0x0003;
const SDP_RETRY_IF_BUSY: c_uint = 0x01;
const SDP_ATTR_REQ_RANGE: c_int = 2;
const SDP_ATTR_SVCNAME_PRIMARY: u16 = 0x0100;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const SOCKET_POLL_TIMEOUT: Duration = Duration::from_millis(250);

pub(crate) struct RfcommServiceQuery<'a> {
    pub service_uuids: &'a [Uuid],
    pub primary_spp_names: &'a [&'a str],
    pub secondary_spp_names: &'a [&'a str],
    pub fallback_channel: Option<u8>,
}

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
pub(super) struct SdpUuid {
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
    fn sdp_uuid128_create(uuid: *mut SdpUuid, value: *const c_void) -> *mut SdpUuid;
    #[cfg(test)]
    fn sdp_uuid2strn(uuid: *const SdpUuid, buf: *mut c_char, length: usize) -> c_int;
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

pub(crate) fn open_rfcomm(
    address: Address,
    query: &RfcommServiceQuery<'_>,
) -> std::io::Result<File> {
    let channels = resolve_rfcomm_channels(address, query)?;
    let mut last_error = None;
    for channel in channels {
        match connect_rfcomm_channel(address, channel) {
            Ok(file) => return Ok(file),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error
        .unwrap_or_else(|| std::io::Error::other(crate::t!("channel-elnk-rfcomm-not-found"))))
}

fn connect_rfcomm_channel(address: Address, channel: u8) -> std::io::Result<File> {
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

fn matches_service_name(name: &str, patterns: &[&str]) -> bool {
    let normalized = name.trim();
    patterns
        .iter()
        .any(|pattern| normalized.eq_ignore_ascii_case(pattern))
}

pub(super) fn create_sdp_uuid128(uuid: Uuid) -> Option<SdpUuid> {
    let mut sdp_uuid = SdpUuid {
        kind: 0,
        value: SdpUuidValue { uuid128: [0; 16] },
    };
    let created = unsafe { sdp_uuid128_create(&mut sdp_uuid, uuid.as_bytes().as_ptr().cast()) };
    if created.is_null() {
        None
    } else {
        Some(sdp_uuid)
    }
}

#[cfg(test)]
pub(super) fn format_sdp_uuid(uuid: &SdpUuid) -> Option<String> {
    let mut buffer = [0 as c_char; 64];
    let result = unsafe { sdp_uuid2strn(uuid, buffer.as_mut_ptr(), buffer.len()) };
    if result < 0 {
        return None;
    }
    Some(
        unsafe { CStr::from_ptr(buffer.as_ptr()) }
            .to_string_lossy()
            .into_owned(),
    )
}

fn resolve_rfcomm_channels(
    address: Address,
    query: &RfcommServiceQuery<'_>,
) -> std::io::Result<Vec<u8>> {
    let fallback_or_err = || -> std::io::Result<Vec<u8>> {
        query
            .fallback_channel
            .map(|ch| vec![ch])
            .ok_or_else(|| std::io::Error::other(crate::t!("channel-elnk-rfcomm-not-found")))
    };

    let local_address = BluetoothAddress([0; 6]);
    let mut remote_bytes = address.0;
    remote_bytes.reverse();
    let remote_address = BluetoothAddress(remote_bytes);
    let session = unsafe { sdp_connect(&local_address, &remote_address, SDP_RETRY_IF_BUSY) };
    if session.is_null() {
        return fallback_or_err();
    }
    let session = SdpSession(session);

    let mut uuid_channels = Vec::new();
    let mut any_query_succeeded = false;

    for &uuid in query.service_uuids {
        let Some(mut sdp_uuid) = create_sdp_uuid128(uuid) else {
            continue;
        };
        if let Some(records) = query_sdp_records(session.0, &mut sdp_uuid) {
            any_query_succeeded = true;
            for (_, channel) in records {
                if (1..=30).contains(&channel) && !uuid_channels.contains(&channel) {
                    uuid_channels.push(channel);
                }
            }
        }
    }

    let mut spp_uuid = SdpUuid {
        kind: 0,
        value: SdpUuidValue { uuid16: 0 },
    };
    let spp_records = if !unsafe { sdp_uuid16_create(&mut spp_uuid, SPP_SERVICE_UUID) }.is_null() {
        if let Some(records) = query_sdp_records(session.0, &mut spp_uuid) {
            any_query_succeeded = true;
            records
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    if !any_query_succeeded {
        return fallback_or_err();
    }

    prioritize_rfcomm_channels(
        &uuid_channels,
        &spp_records,
        query.primary_spp_names,
        query.secondary_spp_names,
        query.fallback_channel,
    )
}

fn query_sdp_records(
    session: *mut c_void,
    service_uuid: &mut SdpUuid,
) -> Option<Vec<(Option<String>, u8)>> {
    let search =
        unsafe { sdp_list_append(std::ptr::null_mut(), (service_uuid as *mut SdpUuid).cast()) };
    if search.is_null() {
        return None;
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
        return None;
    }

    let mut records = std::ptr::null_mut();
    let result = unsafe {
        sdp_service_search_attr_req(
            session,
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
        return None;
    }

    let mut parsed = Vec::new();
    let mut record_node = records;
    while !record_node.is_null() {
        let record = unsafe { (*record_node).data };
        let mut name = [0 as c_char; 257];
        let service_name = if unsafe {
            sdp_get_string_attr(
                record,
                SDP_ATTR_SVCNAME_PRIMARY,
                name.as_mut_ptr(),
                name.len() - 1,
            )
        } == 0
        {
            Some(
                unsafe { CStr::from_ptr(name.as_ptr()) }
                    .to_string_lossy()
                    .into_owned(),
            )
        } else {
            None
        };

        let mut protocols = std::ptr::null_mut();
        if unsafe { sdp_get_access_protos(record, &mut protocols) } == 0 {
            let channel = unsafe { sdp_get_proto_port(protocols, RFCOMM_PROTOCOL_UUID) };
            unsafe { sdp_list_free(protocols, Some(free_nested_sdp_list)) };
            if (1..=30).contains(&channel) {
                parsed.push((service_name, channel as u8));
            }
        }
        unsafe { sdp_record_free(record) };
        record_node = unsafe { (*record_node).next };
    }
    unsafe { sdp_list_free(records, None) };
    Some(parsed)
}

pub(super) fn prioritize_rfcomm_channels(
    uuid_channels: &[u8],
    spp_records: &[(Option<String>, u8)],
    primary_spp_names: &[&str],
    secondary_spp_names: &[&str],
    fallback_channel: Option<u8>,
) -> std::io::Result<Vec<u8>> {
    let mut candidates = Vec::new();
    let push_unique = |candidates: &mut Vec<u8>, channel: u8| {
        if (1..=30).contains(&channel) && !candidates.contains(&channel) {
            candidates.push(channel);
        }
    };

    for &channel in uuid_channels {
        push_unique(&mut candidates, channel);
    }

    let mut primary_channels = Vec::new();
    let mut secondary_channels = Vec::new();
    let mut all_spp_channels = Vec::new();

    for (name, channel) in spp_records {
        if !(1..=30).contains(channel) {
            continue;
        }
        if !all_spp_channels.contains(channel) {
            all_spp_channels.push(*channel);
        }
        if let Some(name) = name {
            if matches_service_name(name, primary_spp_names) {
                if !primary_channels.contains(channel) {
                    primary_channels.push(*channel);
                }
            } else if matches_service_name(name, secondary_spp_names)
                && !secondary_channels.contains(channel)
            {
                secondary_channels.push(*channel);
            }
        }
    }

    if candidates.is_empty() && primary_channels.len() > 1 {
        return Err(std::io::Error::other(crate::t!(
            "channel-sdp-conflicting-rfcomm"
        )));
    }

    for &channel in &primary_channels {
        push_unique(&mut candidates, channel);
    }

    if primary_channels.is_empty() {
        for &channel in &secondary_channels {
            push_unique(&mut candidates, channel);
        }
    }

    if candidates.is_empty() && all_spp_channels.len() == 1 {
        push_unique(&mut candidates, all_spp_channels[0]);
    }

    if candidates.is_empty()
        && let Some(fallback) = fallback_channel
    {
        push_unique(&mut candidates, fallback);
    }

    if candidates.is_empty() {
        return Err(std::io::Error::other(crate::t!(
            "channel-elnk-rfcomm-not-found"
        )));
    }

    Ok(candidates)
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

pub(crate) fn is_socket_timeout(error: &std::io::Error) -> bool {
    matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
}
