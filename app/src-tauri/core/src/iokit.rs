//! Minimal IOKit / IOHID FFI with RAII wrappers. Every pointer is null-checked.
#![allow(non_upper_case_globals, non_camel_case_types)]
use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation_sys::base::{kCFAllocatorDefault, CFAllocatorRef, CFGetTypeID, CFRelease, CFTypeRef};
use core_foundation_sys::dictionary::{CFDictionaryGetValue, CFDictionaryGetTypeID, CFDictionaryRef, CFMutableDictionaryRef};
use core_foundation_sys::number::{
    kCFNumberSInt64Type, CFBooleanGetTypeID, CFBooleanGetValue, CFBooleanRef, CFNumberGetTypeID,
    CFNumberGetValue, CFNumberRef,
};
use core_foundation_sys::string::{CFStringGetTypeID, CFStringRef};
use std::ffi::{c_char, c_void, CString};

type io_object_t = u32;
type kern_return_t = i32;
type mach_port_t = u32;
/// `kIOMainPortDefault` is MACH_PORT_NULL.
const MAIN_PORT: mach_port_t = 0;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOServiceMatching(name: *const c_char) -> CFMutableDictionaryRef;
    fn IOServiceGetMatchingServices(main: mach_port_t, matching: CFDictionaryRef, iter: *mut io_object_t) -> kern_return_t;
    fn IOIteratorNext(iter: io_object_t) -> io_object_t;
    fn IOObjectRelease(obj: io_object_t) -> kern_return_t;
    fn IORegistryEntryCreateCFProperties(
        entry: io_object_t,
        props: *mut CFMutableDictionaryRef,
        allocator: CFAllocatorRef,
        options: u32,
    ) -> kern_return_t;
    fn IORegistryEntryCreateCFProperty(entry: io_object_t, key: CFStringRef, allocator: CFAllocatorRef, options: u32) -> CFTypeRef;

    // IOHID event system (private but stable since 2020; used by Stats, macmon…).
    fn IOHIDEventSystemClientCreate(allocator: CFAllocatorRef) -> *mut c_void;
    fn IOHIDEventSystemClientSetMatching(client: *mut c_void, matching: CFDictionaryRef) -> i32;
    fn IOHIDEventSystemClientCopyServices(client: *mut c_void) -> CFArrayRef;
    fn IOHIDServiceClientCopyProperty(service: *const c_void, key: CFStringRef) -> CFTypeRef;
    fn IOHIDServiceClientCopyEvent(service: *const c_void, kind: i64, options: i32, timestamp: i64) -> *mut c_void;
    fn IOHIDEventGetFloatValue(event: *mut c_void, field: i32) -> f64;
}

/// An owned CF object, released on drop.
pub struct Owned(CFTypeRef);

impl Owned {
    pub fn new(ptr: CFTypeRef) -> Option<Self> {
        (!ptr.is_null()).then_some(Self(ptr))
    }
    pub fn get(&self) -> CFTypeRef {
        self.0
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: we own one reference.
        unsafe { CFRelease(self.0) }
    }
}

pub struct Service(io_object_t);

impl Drop for Service {
    fn drop(&mut self) {
        // SAFETY: we own the io_object.
        unsafe { IOObjectRelease(self.0) };
    }
}

impl Service {
    /// All registry entries of an IOKit class (e.g. "IOBlockStorageDriver").
    pub fn all(class: &str) -> Vec<Service> {
        let Ok(name) = CString::new(class) else { return Vec::new() };
        let mut iter: io_object_t = 0;
        let mut out = Vec::new();
        // SAFETY: IOServiceGetMatchingServices consumes `matching`.
        unsafe {
            let matching = IOServiceMatching(name.as_ptr());
            if matching.is_null() || IOServiceGetMatchingServices(MAIN_PORT, matching, &mut iter) != 0 {
                return out;
            }
            loop {
                let obj = IOIteratorNext(iter);
                if obj == 0 {
                    break;
                }
                out.push(Service(obj));
            }
            IOObjectRelease(iter);
        }
        out
    }

    /// All properties as an owned CFDictionary.
    pub fn properties(&self) -> Option<Owned> {
        let mut props: CFMutableDictionaryRef = std::ptr::null_mut();
        // SAFETY: valid out-pointer; result follows the create rule.
        let r = unsafe { IORegistryEntryCreateCFProperties(self.0, &mut props, kCFAllocatorDefault, 0) };
        if r != 0 {
            return None;
        }
        Owned::new(props as CFTypeRef)
    }

    pub fn property(&self, key: &str) -> Option<Owned> {
        let key = CFString::new(key);
        // SAFETY: create rule.
        Owned::new(unsafe { IORegistryEntryCreateCFProperty(self.0, key.as_concrete_TypeRef(), kCFAllocatorDefault, 0) })
    }
}

/// Borrowed value from a dictionary (valid while the dictionary lives).
pub fn dict_get(dict: CFTypeRef, key: &str) -> Option<CFTypeRef> {
    // SAFETY: type-checked before use.
    unsafe {
        if dict.is_null() || CFGetTypeID(dict) != CFDictionaryGetTypeID() {
            return None;
        }
        let key = CFString::new(key);
        let v = CFDictionaryGetValue(dict as CFDictionaryRef, key.as_concrete_TypeRef() as *const c_void);
        (!v.is_null()).then_some(v as CFTypeRef)
    }
}

pub fn as_i64(v: CFTypeRef) -> Option<i64> {
    // SAFETY: type-checked.
    unsafe {
        if v.is_null() || CFGetTypeID(v) != CFNumberGetTypeID() {
            return None;
        }
        let mut out: i64 = 0;
        CFNumberGetValue(v as CFNumberRef, kCFNumberSInt64Type, (&mut out as *mut i64).cast()).then_some(out)
    }
}

pub fn as_bool(v: CFTypeRef) -> Option<bool> {
    // SAFETY: type-checked.
    unsafe {
        if v.is_null() || CFGetTypeID(v) != CFBooleanGetTypeID() {
            return None;
        }
        Some(CFBooleanGetValue(v as CFBooleanRef))
    }
}

pub fn as_string(v: CFTypeRef) -> Option<String> {
    // SAFETY: type-checked; wrap_under_get_rule retains, so no double free.
    unsafe {
        if v.is_null() || CFGetTypeID(v) != CFStringGetTypeID() {
            return None;
        }
        Some(CFString::wrap_under_get_rule(v as CFStringRef).to_string())
    }
}

pub fn get_i64(dict: CFTypeRef, key: &str) -> Option<i64> {
    dict_get(dict, key).and_then(as_i64)
}

pub fn get_bool(dict: CFTypeRef, key: &str) -> Option<bool> {
    dict_get(dict, key).and_then(as_bool)
}

/// Apple silicon temperature sensors through the IOHID event system.
pub struct HidThermal {
    client: Owned,
    /// (name, service) — services are borrowed from `services`.
    services: Owned,
}

const HID_EVENT_TYPE_TEMPERATURE: i64 = 15;

impl HidThermal {
    pub fn new() -> Option<Self> {
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::number::CFNumber;
        // SAFETY: create rule; matching dict outlives the call.
        unsafe {
            let client = Owned::new(IOHIDEventSystemClientCreate(kCFAllocatorDefault) as CFTypeRef)?;
            let matching = CFDictionary::from_CFType_pairs(&[
                (CFString::new("PrimaryUsagePage"), CFNumber::from(0xff00i32)),
                (CFString::new("PrimaryUsage"), CFNumber::from(5i32)),
            ]);
            IOHIDEventSystemClientSetMatching(client.get() as *mut c_void, matching.as_concrete_TypeRef());
            let services = Owned::new(IOHIDEventSystemClientCopyServices(client.get() as *mut c_void) as CFTypeRef)?;
            Some(Self { client, services })
        }
    }

    /// (sensor name, °C) for every readable sensor.
    pub fn read(&self) -> Vec<(String, f64)> {
        let _keep = &self.client;
        let mut out = Vec::new();
        let key = CFString::new("Product");
        // SAFETY: services array is owned by self; events follow the create rule.
        unsafe {
            let array = self.services.get() as CFArrayRef;
            for i in 0..CFArrayGetCount(array) {
                let service = CFArrayGetValueAtIndex(array, i);
                if service.is_null() {
                    continue;
                }
                let Some(name) = Owned::new(IOHIDServiceClientCopyProperty(service, key.as_concrete_TypeRef())) else {
                    continue;
                };
                let Some(name) = as_string(name.get()) else { continue };
                let Some(event) = Owned::new(IOHIDServiceClientCopyEvent(service, HID_EVENT_TYPE_TEMPERATURE, 0, 0) as CFTypeRef) else {
                    continue;
                };
                let value = IOHIDEventGetFloatValue(event.get() as *mut c_void, (HID_EVENT_TYPE_TEMPERATURE as i32) << 16);
                out.push((name, value));
            }
        }
        out
    }
}
