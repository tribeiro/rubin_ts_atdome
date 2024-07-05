use regex::{Regex, RegexSet};

use crate::atdome_model::ATDomeCmd;

const MOVE_AZ_REGEX: &str = r"(?P<az>[0-9]*) MV";
const CLOSE_SHUTTER_REGEX: &str = r"SC";
const OPEN_SHUTTER_REGEX: &str = r"SO";
const STOP_MOTION_REGEX: &str = r"ST";
const HOME_AZIMUTH_REGEX: &str = r"HM";
const OPEN_SHUTTHER_DROPOUT_REGEX: &str = r"DN";
const CLOSE_SHUTTHER_DROPOUT_REGEX: &str = r"UP";
const OPEN_SHUTTHER_MAIN_DOOR_REGEX: &str = r"OP";
const CLOSE_SHUTTHER_MAIN_DOOR_REGEX: &str = r"CL";
const GET_STATUS_REGEX: &str = r"\+";

pub struct ATDomeCmdRegex {
    regex_set: RegexSet,
    regex: Vec<Regex>,
}

impl ATDomeCmdRegex {
    pub fn new() -> ATDomeCmdRegex {
        let regex_set = RegexSet::new([
            MOVE_AZ_REGEX,
            CLOSE_SHUTTER_REGEX,
            OPEN_SHUTTER_REGEX,
            STOP_MOTION_REGEX,
            HOME_AZIMUTH_REGEX,
            OPEN_SHUTTHER_DROPOUT_REGEX,
            CLOSE_SHUTTHER_DROPOUT_REGEX,
            OPEN_SHUTTHER_MAIN_DOOR_REGEX,
            CLOSE_SHUTTHER_MAIN_DOOR_REGEX,
            GET_STATUS_REGEX,
        ])
        .unwrap();

        let regex = regex_set
            .patterns()
            .into_iter()
            .map(|pattern| Regex::new(pattern).unwrap())
            .collect();

        ATDomeCmdRegex { regex_set, regex }
    }

    fn get_match_index(&self, text: &str) -> Option<usize> {
        self.regex_set.matches(text).into_iter().next()
    }

    pub fn into_atdome_cmd(&self, text: &str) -> ATDomeCmd {
        if let Some(match_index) = self.get_match_index(text) {
            match match_index {
                0 => {
                    let capture = self.regex[match_index].captures(text).unwrap();
                    let az_value: f32 = capture["az"].parse().unwrap();
                    ATDomeCmd::MoveAz(az_value)
                }
                9 => ATDomeCmd::GetStatus,
                1 => ATDomeCmd::CloseShutter,
                2 => ATDomeCmd::OpenShutter,
                3 => ATDomeCmd::StopMotion,
                4 => ATDomeCmd::HomeAzimuth,
                5 => ATDomeCmd::OpenShutterDropoutDoor,
                6 => ATDomeCmd::CloseShutterDropoutDoor,
                7 => ATDomeCmd::OpenShutterMainDoor,
                8 => ATDomeCmd::CloseShutterMainDoor,
                _ => ATDomeCmd::Unknown,
            }
        } else {
            ATDomeCmd::Unknown
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_into_atdome_cmd_move_az() {
        let atdome_cmd_regex = ATDomeCmdRegex::new();

        let atdome_cmd = atdome_cmd_regex.into_atdome_cmd("101 MV");

        assert!(matches!(atdome_cmd, ATDomeCmd::MoveAz(101.0)))
    }

    #[test]
    fn test_into_atdome_cmd_get_status() {
        let atdome_cmd_regex = ATDomeCmdRegex::new();

        let atdome_cmd = atdome_cmd_regex.into_atdome_cmd("+");

        assert!(matches!(atdome_cmd, ATDomeCmd::GetStatus))
    }

    #[test]
    fn test_into_atdome_cmd_close_shutter() {
        let atdome_cmd_regex = ATDomeCmdRegex::new();

        let atdome_cmd = atdome_cmd_regex.into_atdome_cmd("SC");

        assert!(matches!(atdome_cmd, ATDomeCmd::CloseShutter))
    }

    #[test]
    fn test_into_atdome_cmd_open_shutter() {
        let atdome_cmd_regex = ATDomeCmdRegex::new();

        let atdome_cmd = atdome_cmd_regex.into_atdome_cmd("SO");

        assert!(matches!(atdome_cmd, ATDomeCmd::OpenShutter))
    }

    #[test]
    fn test_into_atdome_cmd_stop_motion() {
        let atdome_cmd_regex = ATDomeCmdRegex::new();

        let atdome_cmd = atdome_cmd_regex.into_atdome_cmd("ST");

        assert!(matches!(atdome_cmd, ATDomeCmd::StopMotion))
    }

    #[test]
    fn test_into_atdome_cmd_home_az() {
        let atdome_cmd_regex = ATDomeCmdRegex::new();

        let atdome_cmd = atdome_cmd_regex.into_atdome_cmd("HM");

        assert!(matches!(atdome_cmd, ATDomeCmd::HomeAzimuth))
    }

    #[test]
    fn test_into_atdome_cmd_open_shutter_dropout() {
        let atdome_cmd_regex = ATDomeCmdRegex::new();

        let atdome_cmd = atdome_cmd_regex.into_atdome_cmd("DN");

        assert!(matches!(atdome_cmd, ATDomeCmd::OpenShutterDropoutDoor))
    }

    #[test]
    fn test_into_atdome_cmd_close_shutter_dropout() {
        let atdome_cmd_regex = ATDomeCmdRegex::new();

        let atdome_cmd = atdome_cmd_regex.into_atdome_cmd("UP");

        assert!(matches!(atdome_cmd, ATDomeCmd::CloseShutterDropoutDoor))
    }

    #[test]
    fn test_into_atdome_cmd_open_shutter_main() {
        let atdome_cmd_regex = ATDomeCmdRegex::new();

        let atdome_cmd = atdome_cmd_regex.into_atdome_cmd("OP");

        assert!(matches!(atdome_cmd, ATDomeCmd::OpenShutterMainDoor))
    }

    #[test]
    fn test_into_atdome_cmd_close_shutter_main() {
        let atdome_cmd_regex = ATDomeCmdRegex::new();

        let atdome_cmd = atdome_cmd_regex.into_atdome_cmd("CL");

        assert!(matches!(atdome_cmd, ATDomeCmd::CloseShutterMainDoor))
    }
}
