#[macro_use]
extern crate napi;
#[macro_use]
extern crate napi_derive;

use napi::{CallContext, Result, JsString, Status, Error, JsUnknown, JsFunction, JsUndefined, Module, JsNumber};

use serde_json;
use serde::*;
use std::ffi::*;
use std::convert::{TryInto, TryFrom};

use J2534Common::*;
mod passthru;
use passthru::*;
use J2534Common::PassthruError::ERR_FAILED;

#[derive(Debug, Serialize, Deserialize)]
struct LibError {
  err: String
}

#[derive(Debug, Serialize, Deserialize)]
struct Device {
  dev_id: u32
}

#[derive(Debug, Serialize, Deserialize)]
struct Voltage {
  mv: u32
}

#[js_function]
pub fn get_device_list(mut ctx: CallContext) -> Result<JsUnknown> {
  Ok(match passthru::PassthruDevice::find_all() {
    Ok(dev) => { ctx.env.to_js_value(&dev)? },
    Err(e) =>  ctx.env.to_js_value(&LibError{ err: e.get_err_desc() })?,
  })
}

#[js_function(1)]
pub fn get_version(mut ctx: CallContext) -> Result<JsUnknown> {
  let idx: u32 = u32::try_from(ctx.get::<JsNumber>(0)?)?;
  if passthru::DRIVER.read().unwrap().is_none() {
    return ctx.env.to_js_value(&LibError{ err: "No driver!".to_string() });
  }
  Ok(match &passthru::DRIVER.write().unwrap().as_ref().unwrap().get_version(idx) {
    Ok(v) => ctx.env.to_js_value(v)?,
    Err(e) => ctx.env.to_js_value(&0)?,
  })
}

#[js_function(1)]
pub fn connect_device(mut ctx: CallContext) -> Result<JsUnknown> {
  let v = ctx.get::<JsUnknown>(0)?;
  let deser: PassthruDevice = ctx.env.from_js_value(v)?;

  if passthru::DRIVER.read().unwrap().is_some() {
    return ctx.env.to_js_value(&LibError{ err: "Driver in use!".to_string() });
  }

  match PassthruDrv::load_lib(deser.drv_path) {
    Ok (d) => { // Library load OK?
      match d.open() { // Was the device able to be opened?
        Ok(idx) => { // Yes - Now we keep the library in static ref and return the idx
          DRIVER.write().unwrap().replace(d);
          ctx.env.to_js_value(&Device{ dev_id: idx })
        },
        Err(e) => {
          if e == ERR_FAILED { // Try to get last error
            let err_str = d.get_last_error();
            match err_str {
              Ok(str) => ctx.env.to_js_value(&LibError { err: format!("Operation failed. '{}'", str) }),
              Err(_) => ctx.env.to_js_value(&LibError { err: "Unknown error".to_string() }) // ERR_FAILED but no string??
            }
          } else {
            ctx.env.to_js_value(&LibError { err: e.to_string().to_string() })
          }
        }
      }
    }
    Err(x) => return ctx.env.to_js_value(&LibError{ err: x.to_string() })
  }
}

#[js_function(1)]
pub fn get_vbatt(mut ctx: CallContext) -> Result<JsUnknown> {
  let idx: u32 = u32::try_from(ctx.get::<JsNumber>(0)?)?;
  if passthru::DRIVER.read().unwrap().is_none() {
    return ctx.env.to_js_value(&LibError{ err: "No driver loaded!".to_string() });
  }

  let mut voltage = 0;

  match &passthru::DRIVER.write().unwrap().as_ref().unwrap().ioctl(idx, IoctlID::READ_VBATT, std::ptr::null_mut::<c_void>(), (&mut voltage) as *mut _ as *mut c_void) {
    0x00 => ctx.env.to_js_value(&Voltage{mv: voltage}),
    n => ctx.env.to_js_value(&LibError{ err: n.to_string() })
  }
}


register_module!(ovd, init);

fn init(module: &mut Module) -> Result<()> {
  module.create_named_method("get_device_list", get_device_list)?;
  module.create_named_method("connect_device", connect_device)?;
  module.create_named_method("get_vbatt", get_vbatt)?;
  module.create_named_method("get_version", get_version)?;
  Ok(())
}

#[test]

#[cfg(windows)]
fn test_connect() {
  let tmp = passthru::PassthruDevice::find_all().unwrap();
  let dev = &tmp[1];
  println!("{:?}", dev);
  let res = passthru::PassthruDrv::load_lib(dev.drv_path.clone()).unwrap();
  println!("{:?}", res.open());
}