#[macro_export]
macro_rules! link_section {
    () => {
        #[used]
        #[cfg_attr(target_os = "linux", unsafe(link_section = ".nushell"))]
        #[cfg_attr(target_os = "macos", unsafe(link_section = "__DATA,__nushell"))]
        #[cfg_attr(target_os = "windows", unsafe(link_section = ".nushell"))]
        static NU_COMPAT_VERSION: [u8; 22] = *b"NU_COMPAT_VERSION=0.1\0";
    };
}
