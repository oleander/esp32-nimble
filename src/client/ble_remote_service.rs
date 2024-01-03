use super::ble_client::BLEClientState;
use crate::{
  ble,
  utilities::{voidp_to_ref, ArcUnsafeCell, BleUuid, WeakUnsafeCell},
  BLEAttribute, BLEClient, BLERemoteCharacteristic, BLEReturnCode, Signal,
};
use alloc::vec::Vec;
use core::ffi::c_void;

pub struct BLERemoteServiceState {
  client: WeakUnsafeCell<BLEClientState>,
  uuid: BleUuid,
  start_handle: u16,
  pub(crate) end_handle: u16,
  pub(crate) characteristics: Option<Vec<BLERemoteCharacteristic>>,
  signal: Signal<u32>,
}

impl BLEAttribute for BLERemoteServiceState {
  fn get_client(&self) -> Option<BLEClient> {
    self.client.upgrade().map(BLEClient::from_state)
  }
}

#[derive(Clone)]
pub struct BLERemoteService {
  pub(crate) state: ArcUnsafeCell<BLERemoteServiceState>,
}

impl BLERemoteService {
  pub(crate) fn new(
    client: WeakUnsafeCell<BLEClientState>,
    service: &esp_idf_sys::ble_gatt_svc,
  ) -> Self {
    Self {
      state: ArcUnsafeCell::new(BLERemoteServiceState {
        client,
        uuid: BleUuid::from(service.uuid),
        start_handle: service.start_handle,
        end_handle: service.end_handle,
        characteristics: None,
        signal: Signal::new(),
      }),
    }
  }

  /// Get the service UUID.
  pub fn uuid(&self) -> BleUuid {
    self.state.uuid
  }

  pub async fn get_characteristics(
    &mut self,
  ) -> Result<core::slice::IterMut<'_, BLERemoteCharacteristic>, BLEReturnCode> {
    // if self.state.characteristics.is_none() {
      self.state.characteristics = Some(Vec::new());
      unsafe {
        ble!(esp_idf_sys::ble_gattc_disc_all_chrs(
          self.state.conn_handle(),
          self.state.start_handle,
          self.state.end_handle,
          Some(Self::characteristic_disc_cb),
          self as *mut Self as _,
        ))?;
      }
      ble!(self.state.signal.wait().await)?;
    // }

    Ok(self.state.characteristics.as_mut().unwrap().iter_mut())
  }

  /// Get the characteristic object for the UUID.
  pub async fn get_characteristic(
    &mut self,
    uuid: BleUuid,
  ) -> Result<&mut BLERemoteCharacteristic, BLEReturnCode> {
    let mut iter = self.get_characteristics().await?;
    iter
      .find(|x| x.uuid() == uuid)
      .ok_or_else(|| BLEReturnCode::fail().unwrap_err())
  }

  extern "C" fn characteristic_disc_cb(
    conn_handle: u16,
    error: *const esp_idf_sys::ble_gatt_error,
    chr: *const esp_idf_sys::ble_gatt_chr,
    arg: *mut c_void,
  ) -> i32 {
    let service = unsafe { voidp_to_ref::<Self>(arg) };
    if service.state.conn_handle() != conn_handle {
      return 0;
    }
    let error = unsafe { &*error };
    let chr = unsafe { &*chr };

    if error.status == 0 {
      let chr = BLERemoteCharacteristic::new(ArcUnsafeCell::downgrade(&service.state), chr);
      service.state.characteristics.as_mut().unwrap().push(chr);
      return 0;
    }

    service.state.signal.signal(error.status as _);
    error.status as _
  }
}

impl core::fmt::Debug for BLERemoteService {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "BLERemoteService[{}]", self.state.uuid)?;
    Ok(())
  }
}
