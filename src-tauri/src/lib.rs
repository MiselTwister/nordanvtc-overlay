use serde::Serialize;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use std::thread;
use std::time::Duration;
use tauri::Emitter;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;
#[cfg(target_os = "windows")]
use windows::Win32::System::Memory::{MapViewOfFile, OpenFileMappingW, FILE_MAP_READ};

#[derive(Clone, Serialize)]
struct TelemetryPayload {
    sdkActive: bool,
    speed: f32,
    limit: f32,
    gear: i32,
    fuel: f32,
    fuelRange: f32,
    fuelAvgCons: f32,
    temp: f32,
    damageEngine: f32,
    damageTrans: f32,
    damageCabin: f32,
    damageChassis: f32,
    damageWheels: f32,
    rpm: f32,
    cruiseControl: f32,
    odometer: f32,
    routeDistance: f32,
    routeTime: f32,
    parkBrake: bool,
    airPressureEmerg: bool,
    oilPressWarning: bool,
    waterTempWarning: bool,
    battVoltWarning: bool,
}

#[cfg(target_os = "windows")]
fn to_wstring(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            #[cfg(target_os = "windows")]
            thread::spawn(move || {
                let map_name = to_wstring("Local\\SCSTelemetry");

                unsafe {
                    let mut handle: HANDLE = HANDLE::default();
                    
                    loop {
                        handle = OpenFileMappingW(
                            FILE_MAP_READ.0, 
                            false,
                            windows::core::PCWSTR(map_name.as_ptr()),
                        ).unwrap_or(HANDLE::default());

                        if !handle.is_invalid() { break; }
                        thread::sleep(Duration::from_secs(2));
                    }

                    let map_ptr = MapViewOfFile(handle, FILE_MAP_READ, 0, 0, 0);
                    if map_ptr.Value.is_null() { return; }

                    let base_ptr = map_ptr.Value as *const u8;
                    
                    // 🛠️ TIME FREEZE VARIABLES
                    let mut last_time: u64 = 0;
                    let mut unchanged_counter: i32 = 0;

                    loop {
                        let sdk_active_raw = ptr::read_unaligned(base_ptr.add(0) as *const u8) != 0;
                        let current_time = ptr::read_unaligned(base_ptr.add(8) as *const u64);
                        
                        let mut is_active = sdk_active_raw;

                        // 🛠️ TIME FREEZE DETECTOR: Hides HUD if game is closed or paused
                        if current_time == last_time && current_time != 0 {
                            unchanged_counter += 1;
                            if unchanged_counter > 30 { // Approx 1 second of frozen time
                                is_active = false;
                            }
                        } else {
                            unchanged_counter = 0;
                        }
                        last_time = current_time;

                        let gear_ptr = base_ptr.add(504) as *const i32;
                        let fuel_capacity_ptr = base_ptr.add(704) as *const f32;
                        let speed_ptr = base_ptr.add(948) as *const f32;
                        let rpm_ptr = base_ptr.add(952) as *const f32;
                        let cruise_ptr = base_ptr.add(988) as *const f32;
                        let fuel_liters_ptr = base_ptr.add(1000) as *const f32;
                        let fuel_cons_ptr = base_ptr.add(1004) as *const f32;
                        let fuel_range_ptr = base_ptr.add(1008) as *const f32;
                        let water_temp_ptr = base_ptr.add(1024) as *const f32;
                        
                        let wear_engine = base_ptr.add(1036) as *const f32;
                        let wear_trans = base_ptr.add(1040) as *const f32;
                        let wear_cabin = base_ptr.add(1044) as *const f32;
                        let wear_chassis = base_ptr.add(1048) as *const f32;
                        let wear_wheels = base_ptr.add(1052) as *const f32;

                        let odometer_ptr = base_ptr.add(1056) as *const f32;
                        let route_dist_ptr = base_ptr.add(1060) as *const f32;
                        let route_time_ptr = base_ptr.add(1064) as *const f32;
                        let speed_limit_ptr = base_ptr.add(1068) as *const f32;

                        // Warning Bools
                        let park_brake = ptr::read_unaligned(base_ptr.add(1566) as *const u8) != 0;
                        let air_emerg = ptr::read_unaligned(base_ptr.add(1569) as *const u8) != 0;
                        let oil_warn = ptr::read_unaligned(base_ptr.add(1572) as *const u8) != 0;
                        let water_warn = ptr::read_unaligned(base_ptr.add(1573) as *const u8) != 0;
                        let batt_warn = ptr::read_unaligned(base_ptr.add(1574) as *const u8) != 0;

                        let fuel_capacity = ptr::read_unaligned(fuel_capacity_ptr).max(1.0); 
                        let fuel_liters = ptr::read_unaligned(fuel_liters_ptr);

                        let payload = TelemetryPayload {
                            sdkActive: is_active, // Uses the frozen time logic!
                            speed: ptr::read_unaligned(speed_ptr), 
                            limit: ptr::read_unaligned(speed_limit_ptr), 
                            gear: ptr::read_unaligned(gear_ptr),
                            fuel: (fuel_liters / fuel_capacity) * 100.0,
                            fuelRange: ptr::read_unaligned(fuel_range_ptr) / 1000.0, 
                            fuelAvgCons: ptr::read_unaligned(fuel_cons_ptr),
                            temp: ptr::read_unaligned(water_temp_ptr),
                            
                            damageEngine: ptr::read_unaligned(wear_engine) * 100.0,
                            damageTrans: ptr::read_unaligned(wear_trans) * 100.0,
                            damageCabin: ptr::read_unaligned(wear_cabin) * 100.0,
                            damageChassis: ptr::read_unaligned(wear_chassis) * 100.0,
                            damageWheels: ptr::read_unaligned(wear_wheels) * 100.0,

                            rpm: ptr::read_unaligned(rpm_ptr),
                            cruiseControl: ptr::read_unaligned(cruise_ptr), 
                            odometer: ptr::read_unaligned(odometer_ptr),
                            routeDistance: ptr::read_unaligned(route_dist_ptr) / 1000.0, 
                            routeTime: ptr::read_unaligned(route_time_ptr),

                            parkBrake: park_brake,
                            airPressureEmerg: air_emerg,
                            oilPressWarning: oil_warn,
                            waterTempWarning: water_warn,
                            battVoltWarning: batt_warn,
                        };

                        app_handle.emit("telemetry-update", payload).unwrap();
                        thread::sleep(Duration::from_millis(33));
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}