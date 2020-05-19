#![allow(non_camel_case_types, unused)]

use crate::document::Document;

extern fn js_init(env: napi_env, exports: napi_value) -> napi_value {
    silly!("init native module");

    unsafe { crate::window::init() };

    start_wakeup_thread();

    env.set_prop(exports, "waitEvents", env.create_fn(js_wait_events));
    env.set_prop(exports, "createWindow", env.create_fn(js_create_window));
    env.set_prop(exports, "createDocument", env.create_fn(js_create_document));
    env.set_prop(exports, "createElement", env.create_fn(js_create_element));

    exports
}

extern fn js_wait_events(env: napi_env, cb_info: napi_callback_info) -> napi_value {
    // wait/poll depending on how far is the next "tick"
    let timeout_ms = match unsafe { uv_backend_timeout(uv_default_loop()) } {
        -1 => None,
        n => Some(n)
    };

    unsafe { crate::window::wait_events(timeout_ms) };

    env.undefined()
}

extern fn js_create_window(env: napi_env, cb_info: napi_callback_info) -> napi_value {
    let [title, width, height] = env.args(cb_info);

    unsafe { crate::window::create_window(&env.string(title), env.i32(width), env.i32(height)) };

    env.undefined()
}

//extern fn js_set_window_listener

extern fn js_create_document(env: napi_env, cb_info: napi_callback_info) -> napi_value {
    env.create_box(Box::new(Document::empty_html()))
}

extern fn js_create_element(env: napi_env, cb_info: napi_callback_info) -> napi_value {
    let [doc, tag_name, _] = env.args(cb_info);
    let el = unsafe { env.downcast_mut::<Document>(doc).create_element(&env.string(tag_name)) };

    env.create_box(Box::new(el))
}
























// wait for I/O and awake the main thread which should in turn
// return back to node and handle it
//
// I think electron is doing something similar but their approach
// seems to be much more complicated (and maybe better)
//
// TODO: windows, linux
fn start_wakeup_thread() {
    std::thread::spawn(move || {
        let node_fd = unsafe { uv_backend_fd(uv_default_loop()) };
        assert_ne!(node_fd, -1, "couldnt get uv_loop fd");

        loop {
            let mut ev = unsafe { std::mem::zeroed::<kevent>() };

            match unsafe { kevent(node_fd, std::ptr::null(), 0, &mut ev, 1, null()) } {
                // shouldn't happen
                0 => eprintln!("kevent returned early"),

                -1 => {
                    eprintln!("kevent err");
                    return;
                }

                // something's pending (res is NOT number of pending events)
                _ => {
                    silly!("pending I/O, waking up UI thread");
                    unsafe { crate::window::wakeup() };

                    // let nodejs handle it first then we can wait again
                    std::thread::sleep(std::time::Duration::from_millis(100))
                }
            }
        }
    });

    extern {
      fn kevent(kq: c_int, changelist: *const kevent, nchanges: c_int, eventlist: *mut kevent, nevents: c_int, timeout: *const timespec) -> c_int;
    }

    #[repr(C)]
    struct kevent {
        pub ident: usize,
        pub filter: i16,
        pub flags: u16,
        pub fflags: u32,
        pub data: isize,
        pub udata: *mut c_void,
    }

    #[repr(C)]
    struct timespec {
        pub tv_sec: i64,
        pub tv_nsec: i64,
    }
}











use std::ptr::{null, null_mut};
use std::os::raw::{c_char, c_int, c_uint, c_void};

#[repr(C)]
#[derive(Debug, PartialEq)]
#[allow(unused)]
enum napi_status {
    Ok,
    InvalidArg,
    ObjectExpected,
    StringExpected,
    NameExpected,
    FunctionExpected,
    NumberExpected,
    BooleanExpected,
    ArrayExpected,
    GenericFailure,
    PendingException,
    Cancelled,
    EscapeCalledTwice,
    HandleScopeMismatch,
}

type napi_value = *const c_void;
type napi_callback = unsafe extern "C" fn(napi_env, napi_callback_info) -> napi_value;
type napi_callback_info = *const c_void;
type napi_finalize = unsafe extern "C" fn(napi_env, *mut c_void, *mut c_void);

const NAPI_AUTO_LENGTH: usize = usize::max_value();

#[repr(C)]
#[derive(Clone, Copy)]
struct napi_env(*const c_void);

// call napi with empty value, check status & return result
// it should be safe but putting unsafe around it would supress
// unsafe warnings for arg expressions too
macro_rules! get_res {
    ($env:expr, $napi_fn:ident $($arg:tt)*) => {{
        let mut res_value = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
        let res = $napi_fn($env $($arg)*, &mut res_value);

        assert_eq!(res, napi_status::Ok);

        res_value
    }}
}

impl napi_env {
    fn undefined(&self) -> napi_value {
        unsafe { get_res!(*self, napi_get_undefined) }
    }

    fn i32(&self, v: napi_value) -> i32 {
        unsafe { get_res!(*self, napi_get_value_int32, v) }
    }

    // V8 strings can be encoded in many ways so we NEED to convert them
    // (https://stackoverflow.com/questions/40512393/understanding-string-heap-size-in-javascript-v8)
    fn string(&self, v: napi_value) -> String {
        unsafe {
            let len = get_res!(*self, napi_get_value_string_utf8, v, null_mut(), 0);

            // +1 because of \0
            let mut bytes = Vec::with_capacity(len + 1);
            get_res!(*self, napi_get_value_string_utf8, v, bytes.as_mut_ptr() as *mut c_char, len + 1);

            // (capacity vs len)
            bytes.set_len(len);

            String::from_utf8_unchecked(bytes)
        }
    }

    // very unsafe but I couldn't get it working with Any
    // maybe double-boxing could work?
    // but then we could just do own (tag + payload encoding)
    unsafe fn downcast_mut<T>(&self, v: napi_value) -> &mut T {
        let ptr = get_res!(*self, napi_get_value_external, v) as *mut T;

        std::mem::transmute(ptr)
    }

    // for simplicity, we always expect 3 args
    // (it's easy to _ any of them and hopefully 3 could be enough)
    fn args(&self, cb_info: napi_callback_info) -> [napi_value; 3] {
        unsafe {
            let mut argv = [std::mem::zeroed(); 3];
            let mut argc = argv.len();
            let mut this_arg = std::mem::zeroed();
            napi_get_cb_info(*self, cb_info, &mut argc, &mut argv[0], &mut this_arg, null_mut());

            argv
        }
    }

    fn create_fn(&self, f: napi_callback) -> napi_value {
        unsafe { get_res!(*self, napi_create_function, null(), NAPI_AUTO_LENGTH, f, null()) }
    }

    fn create_box<T>(&self, v: Box<T>) -> napi_value {
        unsafe { get_res!(*self, napi_create_external, Box::into_raw(v) as *const c_void, Self::drop_box::<T>, null()) }
    }

    fn set_prop(&self, target: napi_value, key: &str, value: napi_value) {
        assert_eq!(unsafe { napi_set_named_property(*self, target, c_str!(key), value) }, napi_status::Ok)
    }

    // has to be generic
    // (own impl for each type we pass to create_box)
    unsafe extern fn drop_box<T>(env: napi_env, data: *mut c_void, hint: *mut c_void) {
        Box::from_raw(data as *mut T);
    }
}



/*
// node.js bindings


*/

dylib! {
    #[load_node_api]
    extern "C" {
        fn napi_module_register(module: *mut napi_module) -> napi_status;
        fn napi_set_named_property(env: napi_env, object: napi_value, utf8name: *const c_char, value: napi_value) -> napi_status;

        fn napi_get_undefined(env: napi_env, result: *mut napi_value) -> napi_status;
        fn napi_get_value_int32(env: napi_env, value: napi_value, result: *mut c_int) -> napi_status;
        fn napi_get_value_string_utf8(env: napi_env, value: napi_value, buf: *mut c_char, bufsize: usize, result: *mut usize) -> napi_status;
        fn napi_get_value_external(env: napi_env, value: napi_value, result: *mut *mut c_void) -> napi_status;

        fn napi_create_function(env: napi_env, utf8name: *const c_char, length: usize, cb: napi_callback, data: *const c_void, result: *mut napi_value) -> napi_status;
        fn napi_create_external(env: napi_env, data: *const c_void, finalize: napi_finalize, finalize_hint: *const c_void, result: *mut napi_value) -> napi_status;
        fn napi_create_array(env: napi_env, result: *mut napi_value) -> napi_status;
        fn napi_set_element(env: napi_env, arr: napi_value, index: c_uint, value: napi_value) -> napi_status;

        fn napi_get_cb_info(env: napi_env, cb_info: napi_callback_info, argc: *mut usize, argv: *mut napi_value, this_arg: *mut napi_value, data: *mut c_void) -> napi_status;




        fn uv_default_loop() -> *const c_void;
        fn uv_backend_fd(uv_loop: *const c_void) -> c_int;
        fn uv_backend_timeout(uv_loop: *const c_void) -> c_int;


        fn napi_get_value_uint32(env: napi_env, napi_value: napi_value, result: *mut c_uint) -> napi_status;
        fn napi_get_value_double(env: napi_env, napi_value: napi_value, result: *mut f64) -> napi_status;
        fn napi_get_value_bool(env: napi_env, napi_value: napi_value, result: *mut bool) -> napi_status;

        fn napi_create_uint32(env: napi_env, value: c_uint, result: *mut napi_value) -> napi_status;
        fn napi_create_int32(env: napi_env, value: c_int, result: *mut napi_value) -> napi_status;
        fn napi_create_double(env: napi_env, value: f64, result: *mut napi_value) -> napi_status;
    }
}

#[repr(C)]
struct napi_module {
    nm_version: c_int,
    nm_flags: c_uint,
    nm_filename: *const c_char,
    nm_register_func: unsafe extern "C" fn(napi_env, napi_value) -> napi_value,
    nm_modname: *const c_char,
    nm_priv: *const c_void,
    reserved: [*const c_void; 4],
}

#[no_mangle]
#[cfg_attr(target_os = "linux", link_section = ".ctors")]
#[cfg_attr(target_os = "macos", link_section = "__DATA,__mod_init_func")]
#[cfg_attr(target_os = "windows", link_section = ".CRT$XCU")]
static REGISTER_NODE_MODULE: unsafe extern "C" fn() = {
    static mut NAPI_MODULE: napi_module = napi_module {
        nm_version: 1,
        nm_flags: 0,
        nm_filename: c_str!("nodejs.rs"),
        nm_register_func: js_init,
        nm_modname: c_str!("libgraffiti"),
        nm_priv: null(),
        reserved: [null(); 4],
    };

    unsafe extern "C" fn register_node_module() {
        silly!("loading node api");
        load_node_api(if cfg!(target_os = "windows") { c_str!("node.exe") } else { null() });

        silly!("calling napi_module_register");
        napi_module_register(&mut NAPI_MODULE);
    }

    register_node_module
};
