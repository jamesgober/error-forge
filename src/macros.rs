/// Global error hook for centralized error handling
static mut ERROR_HOOK: Option<fn(&str)> = None;

/// Register a callback function to be called when errors are created
/// 
/// # Safety
/// This function is unsafe because it modifies a global static variable
pub fn register_error_hook(callback: fn(&str)) {
    unsafe { ERROR_HOOK = Some(callback); }
}

/// Call the registered error hook with a message if one is registered
#[doc(hidden)]
pub fn call_error_hook(message: &str) {
    unsafe {
        if let Some(hook) = ERROR_HOOK {
            hook(message);
        }
    }
}

#[macro_export]
macro_rules! define_errors {
    (
        $(
            $(#[$meta:meta])* $vis:vis enum $name:ident {
                $( 
                   $(#[error(display = $display:literal $(, $($display_param:ident),* )?)])?
                   #[kind($kind:ident $(, $($tag:ident = $val:expr),* )?)]
                   $variant:ident $( { $($field:ident : $ftype:ty),* $(,)? } )?, )*
            }
        )*
    ) => {
        $(
            $(#[$meta])* #[derive(Debug)]
            #[cfg_attr(feature = "serde", derive(serde::Serialize))]
            $vis enum $name {
                $( $variant $( { $($field : $ftype),* } )?, )*
            }

            impl $name {
                $(
                    paste::paste! {
                        pub fn [<$variant:lower>]($($($field : $ftype),*)?) -> Self {
                            let instance = Self::$variant $( { $($field),* } )?;
                            #[allow(unsafe_code)]
                            unsafe {
                                if let Some(hook) = $crate::macros::ERROR_HOOK {
                                    let _ = hook(instance.caption());
                                }
                            }
                            instance
                        }
                    }
                )*

                pub fn caption(&self) -> &'static str {
                    match self {
                        $( Self::$variant { .. } => {
                            define_errors!(@get_caption $kind $(, $($tag = $val),* )?)
                        } ),*
                    }
                }
                
                pub fn kind(&self) -> &'static str {
                    match self {
                        $( Self::$variant { .. } => {
                            stringify!($kind)
                        } ),*
                    }
                }

                pub fn is_retryable(&self) -> bool {
                    match self {
                        $( Self::$variant { .. } => {
                            define_errors!(@get_tag retryable false $(, $($tag = $val),* )?)
                        } ),*
                    }
                }

                pub fn is_fatal(&self) -> bool {
                    match self {
                        $( Self::$variant { .. } => {
                            define_errors!(@get_tag fatal true $(, $($tag = $val),* )?)
                        } ),*
                    }
                }

                pub fn status_code(&self) -> u16 {
                    match self {
                        $( Self::$variant { .. } => {
                            define_errors!(@get_tag status 500 $(, $($tag = $val),* )?)
                        } ),*
                    }
                }

                pub fn exit_code(&self) -> i32 {
                    match self {
                        $( Self::$variant { .. } => {
                            define_errors!(@get_tag exit 1 $(, $($tag = $val),* )?)
                        } ),*
                    }
                }
            }

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        $( Self::$variant $( { $($field),* } )? => {
                            $(                                
                                #[allow(unused_variables)]
                                if let Some(display) = define_errors!(@format_display $display $(, $($display_param),*)?) {
                                    return write!(f, "{}", display);
                                }
                            )?
                            // If no custom display format is provided, use a default format
                            write!(f, "{}: ", self.caption())?;
                            write!(f, stringify!($variant))?;
                            // Format each field with name=value
                            $( $( 
                                write!(f, " | {} = ", stringify!($field))?
                                ;
                                match stringify!($field) {
                                    "source" => write!(f, "{}", $field)?,
                                    _ => write!(f, "{:?}", $field)?,
                                }
                            ; )* )?
                            Ok(())
                        } ),*
                    }
                }
            }

            impl std::error::Error for $name {
                fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                    match self {
                        $( Self::$variant $( { $($field),* } )? => {
                            define_errors!(@find_source $($field),*)
                        } ),*
                    }
                }
            }
        )*
    };

    (@find_source) => {
        None
    };
    
    (@find_source $field:ident) => {
        {
            let val = $field;
            if let Some(err) = (val as &dyn std::any::Any).downcast_ref::<&(dyn std::error::Error + 'static)>() {
                Some(*err)
            } else {
                None
            }
        }
    };
    
    (@find_source $field:ident, $($rest:ident),+) => {
        {
            let val = $field;
            if let Some(err) = (val as &dyn std::any::Any).downcast_ref::<&(dyn std::error::Error + 'static)>() {
                Some(*err)
            } else {
                define_errors!(@find_source $($rest),+)
            }
        }
    };

    (@get_caption $kind:ident $(, caption = $caption:expr $(, $($rest:tt)*)? )?) => {
        $crate::define_errors!(@unwrap_caption $kind $(, $caption)? )
    };

    (@unwrap_caption Config, $caption:expr) => { $caption };
    (@unwrap_caption Filesystem, $caption:expr) => { $caption };
    (@unwrap_caption $kind:ident) => { stringify!($kind) };

    (@get_tag $target:ident, $default:expr $(, $($tag:ident = $val:expr),* )?) => {
        {
            let mut found = $default;
            $( $( if stringify!($tag) == stringify!($target) { found = $val; })* )?
            found
        }
    };
    
    (@format_display $display:literal $(, $($param:ident),*)?) => {
        {
            // When parameters are provided, use them for formatting
            $(
                Some(format!($display, $($param = $param),*))
            )?
            // When no parameters are provided, just use the literal string
            $(
                Some($display.to_string())
            )?
        }
    };

    // Support for nested field access in error display formatting
    (@format_display_field $field:ident) => {
        $field
    };

    (@format_display_field $field:ident . $($rest:ident).+) => {
        $field$(.$rest)+
    };
}
