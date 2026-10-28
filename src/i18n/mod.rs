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

pub use fluent_templates::fluent_bundle::FluentValue;
use fluent_templates::{Loader, static_loader};
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "en-US",
        customise: |bundle| bundle.set_use_isolating(false),
    };
}

thread_local! {
    static THREAD_LOCALE: std::cell::RefCell<Option<LanguageIdentifier>> = const { std::cell::RefCell::new(None) };
}

static CONFIGURED_LOCALE: std::sync::RwLock<Option<LanguageIdentifier>> =
    std::sync::RwLock::new(None);

pub fn set_locale(locale_str: &str) {
    let parsed = parse_locale(locale_str);
    THREAD_LOCALE.with(|tl| *tl.borrow_mut() = parsed);
}

pub fn reset_locale() {
    THREAD_LOCALE.with(|tl| *tl.borrow_mut() = None);
}

pub fn current_locale() -> LanguageIdentifier {
    let thread_override = THREAD_LOCALE.with(|tl| tl.borrow().clone());
    if let Some(lang) = thread_override {
        return lang;
    }

    if let Ok(guard) = CONFIGURED_LOCALE.read() {
        if let Some(ref lang) = *guard {
            return lang.clone();
        }
    }

    if let Ok(env_lang) = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
    {
        if let Some(parsed) = parse_locale(&env_lang) {
            return parsed;
        }
    }

    if let Some(sys_lang) = sys_locale::get_locale() {
        if let Some(parsed) = parse_locale(&sys_lang) {
            return parsed;
        }
    }

    "en-US".parse().unwrap()
}

fn parse_locale(s: &str) -> Option<LanguageIdentifier> {
    let clean = s.split('.').next()?.split('@').next()?.trim();
    if clean.is_empty() || clean == "C" || clean == "POSIX" {
        return None;
    }
    let bcp47 = clean.replace('_', "-");
    bcp47.parse().ok().or_else(|| {
        let prefix = bcp47.split('-').next()?;
        prefix.parse().ok()
    })
}

fn config_file_path() -> std::path::PathBuf {
    gtk::glib::user_config_dir()
        .join("wavetune")
        .join("settings.ini")
}

pub fn get_configured_language() -> Option<String> {
    let path = config_file_path();
    if !path.exists() {
        return None;
    }
    let keyfile = gtk::glib::KeyFile::new();
    if keyfile
        .load_from_file(&path, gtk::glib::KeyFileFlags::NONE)
        .is_ok()
    {
        if let Ok(lang) = keyfile.string("General", "language") {
            let lang = lang.trim();
            if !lang.is_empty() && lang != "auto" {
                return Some(lang.to_string());
            }
        }
    }
    None
}

pub fn set_configured_language(lang_opt: Option<&str>) {
    let path = config_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let keyfile = gtk::glib::KeyFile::new();
    let _ = keyfile.load_from_file(&path, gtk::glib::KeyFileFlags::NONE);

    let val = match lang_opt {
        Some(s) if !s.trim().is_empty() && s != "auto" => s.trim(),
        _ => "auto",
    };
    keyfile.set_string("General", "language", val);
    let _ = keyfile.save_to_file(&path);

    let parsed = if val == "auto" {
        None
    } else {
        parse_locale(val)
    };
    if let Ok(mut guard) = CONFIGURED_LOCALE.write() {
        *guard = parsed;
    }
}

pub fn init() {
    if let Some(lang) = get_configured_language() {
        if let Some(parsed) = parse_locale(&lang) {
            if let Ok(mut guard) = CONFIGURED_LOCALE.write() {
                *guard = Some(parsed);
            }
        }
    }
}

pub fn tr(key: &str) -> String {
    let lang = current_locale();
    LOCALES
        .try_lookup(&lang, key)
        .unwrap_or_else(|| key.to_string())
}

pub fn tr_args(
    key: &str,
    args: &HashMap<
        std::borrow::Cow<'static, str>,
        fluent_templates::fluent_bundle::FluentValue<'_>,
    >,
) -> String {
    let lang = current_locale();
    LOCALES
        .try_lookup_with_args(&lang, key, args)
        .unwrap_or_else(|| key.to_string())
}

pub trait ToFluentValue {
    fn to_fluent_value(&self) -> FluentValue<'static>;
}

impl ToFluentValue for &str {
    fn to_fluent_value(&self) -> FluentValue<'static> {
        FluentValue::from((*self).to_string())
    }
}

impl ToFluentValue for str {
    fn to_fluent_value(&self) -> FluentValue<'static> {
        FluentValue::from(self.to_string())
    }
}

impl ToFluentValue for String {
    fn to_fluent_value(&self) -> FluentValue<'static> {
        FluentValue::from(self.clone())
    }
}

impl ToFluentValue for &String {
    fn to_fluent_value(&self) -> FluentValue<'static> {
        FluentValue::from((*self).clone())
    }
}

impl<T> ToFluentValue for std::sync::mpsc::SendError<T> {
    fn to_fluent_value(&self) -> FluentValue<'static> {
        FluentValue::from(self.to_string())
    }
}

impl ToFluentValue for std::io::Error {
    fn to_fluent_value(&self) -> FluentValue<'static> {
        FluentValue::from(self.to_string())
    }
}

impl ToFluentValue for &std::io::Error {
    fn to_fluent_value(&self) -> FluentValue<'static> {
        FluentValue::from(self.to_string())
    }
}

macro_rules! impl_to_fluent_value_num {
    ($($t:ty),*) => {
        $(
            impl ToFluentValue for $t {
                fn to_fluent_value(&self) -> FluentValue<'static> {
                    FluentValue::from(*self as i64)
                }
            }
            impl ToFluentValue for &$t {
                fn to_fluent_value(&self) -> FluentValue<'static> {
                    FluentValue::from(**self as i64)
                }
            }
        )*
    };
}

impl_to_fluent_value_num!(u8, u16, u32, usize, i8, i16, i32, i64, isize);

#[macro_export]
macro_rules! fluent_args {
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut map = ::std::collections::HashMap::<::std::borrow::Cow<'static, str>, $crate::i18n::FluentValue<'static>>::new();
        $(
            map.insert(::std::borrow::Cow::Borrowed($key), $crate::i18n::ToFluentValue::to_fluent_value(&$val));
        )*
        map
    }};
}

#[macro_export]
macro_rules! t {
    ($id:expr) => {
        $crate::i18n::tr($id)
    };
    ($id:expr, $($key:ident = $val:expr),+ $(,)?) => {{
        let args = $crate::fluent_args! {
            $(stringify!($key) => $val),+
        };
        $crate::i18n::tr_args($id, &args)
    }};
    ($id:expr, $($val:expr),+ $(,)?) => {{
        #[allow(unused_assignments)]
        let mut idx = 0;
        let mut map = ::std::collections::HashMap::<::std::borrow::Cow<'static, str>, $crate::i18n::FluentValue<'static>>::new();
        $(
            map.insert(::std::borrow::Cow::Owned(format!("param{idx}")), $crate::i18n::ToFluentValue::to_fluent_value(&$val));
            idx += 1;
        )+
        let _ = idx;
        $crate::i18n::tr_args($id, &map)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fluent_falls_back_to_english_when_locale_is_en() {
        set_locale("en-US");
        assert_eq!(t!("nav-home"), "Home");
        assert_eq!(t!("common-connect"), "Connect");
        assert_eq!(t!("noise-mode-deep"), "Deep");
        assert_eq!(t!("equalizer-preset-dynaudio-classic"), "Dynaudio Classic");
        assert_eq!(t!("pref-preferences"), "Preferences");
        assert_eq!(t!("about-title"), "About WaveTune");
        assert_eq!(t!("pref-language"), "Language");
        assert_eq!(t!("common-status", param0 = "OK"), "Status: OK");
        assert_eq!(t!("common-status", "OK"), "Status: OK");
        reset_locale();
    }

    #[test]
    fn fluent_translates_chinese_when_locale_is_zh_cn() {
        set_locale("zh-CN");
        assert_eq!(t!("nav-home"), "主页");
        assert_eq!(t!("common-connect"), "连接");
        assert_eq!(t!("noise-mode-deep"), "深度降噪");
        assert_eq!(t!("equalizer-preset-dynaudio-classic"), "丹拿原声");
        assert_eq!(t!("pref-preferences"), "首选项");
        assert_eq!(t!("about-title"), "关于 WaveTune");
        assert_eq!(t!("pref-language"), "语言");
        assert_eq!(t!("common-status", param0 = "正常"), "状态：正常");
        assert_eq!(t!("common-status", "正常"), "状态：正常");
        reset_locale();
    }

    #[test]
    fn untranslated_key_returns_key_itself() {
        set_locale("en-US");
        assert_eq!(t!("nonexistent-key-12345"), "nonexistent-key-12345");
        reset_locale();
    }

    /// libadwaita parses `PreferencesGroup`/`PreferencesRow` titles as Pango markup; an invalid
    /// value (e.g. a bare `&`) silently renders as an empty label.
    #[test]
    fn all_translations_are_valid_pango_markup() {
        let files = [
            ("en-US", include_str!("../../locales/en-US/wavetune.ftl")),
            ("zh-CN", include_str!("../../locales/zh-CN/wavetune.ftl")),
        ];
        let mut failures = Vec::new();
        for (locale, source) in files {
            for (line_no, line) in source.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                let value = value.trim();
                if let Err(err) = gtk::pango::parse_markup(value, '\0') {
                    failures.push(format!(
                        "{locale}:{} `{}` = {value:?}: {err}",
                        line_no + 1,
                        key.trim()
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "translations with invalid Pango markup (escape `&` as `&amp;` or reword):\n{}",
            failures.join("\n")
        );
    }
}
