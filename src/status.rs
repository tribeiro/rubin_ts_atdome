//! Define the Status struct, representing all information available from the ATDome controller.

#[derive(Debug, Default, Clone, Copy)]
pub struct Status {
    pub auto_shutdown_enabled: bool,
    pub az_home_switch: bool,
    pub az_pos: f32,
    pub azimuth_move_timeout: f32,
    pub cloud_sensor_enabled: bool,
    pub coast: f32,
    pub door_move_timeout: f32,
    pub dropout_door_encoder_closed: u64,
    pub dropout_door_encoder_opened: u64,
    pub dropout_door_pct: f32,
    pub dropout_timer: f32,
    pub encoder_counts: u64,
    pub encoder_counts_per_360: u64,
    pub estop_active: bool,
    pub high_speed: f32,
    pub home_azimuth: f32,
    pub homed: bool,
    pub last_azimuth_goto: f32,
    pub main_door_encoder_closed: u64,
    pub main_door_encoder_opened: u64,
    pub main_door_pct: f32,
    pub move_code: u8,
    pub rain_sensor_enabled: bool,
    pub reversal_delay: f32,
    pub scb_link_ok: bool,
    pub sensor_code: usize,
    pub tolerance: f32,
    pub watchdog_timer: f32,
}

impl Status {
    pub fn as_string(&self) -> String {
        format!(
            "MAIN CLOSED 000
DROP CLOSED 000
[OFF] 00
POSN {}
-- {:03}
Dome not homed
Emergency Stop Active: 0
Top Comm Link OK:    1
Home Azimuth: 10.00
High Speed (degrees):  5.00
Coast (degrees): 0.50
Tolerance (degrees): 1.00
Encoder Counts per 360: 4018143232
Encoder Counts:  111615089
Last Azimuth GoTo: {}
Azimuth Move Timeout (secs): 120
Rain-Snow enabled:  1
Cloud Sensor enabled: 1
Watchdog Reset Time: 600
Dropout Timer: 5
Reverse Delay: 4
Main Door Encoder Closed: 118449181478
Main Door Encoder Opened: 8287616388
Dropout Encoder Closed: 5669776578
Dropout Encoder Opened: 5710996184
Door Move Timeout (secs): 360
Dome has been homed: False
",
            self.az_pos, self.move_code, self.last_azimuth_goto,
        )
    }
}
