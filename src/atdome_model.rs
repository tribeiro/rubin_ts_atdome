//! Provide an interface to the ATDome Controller.

use crate::{
    error::{ATDomeError, ATDomeResult},
    status::Status,
    status_parser::StatusParser,
};
use std::str;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{mpsc, oneshot},
    task,
};

#[derive(Debug)]
pub enum ATDomeCmd {
    MoveAz(f32),
    CloseShutter,
    OpenShutter,
    StopMotion,
    HomeAzimuth,
    OpenShutterDropoutDoor,
    CloseShutterDropoutDoor,
    OpenShutterMainDoor,
    CloseShutterMainDoor,
    GetStatus,
    Unknown,
}

impl ATDomeCmd {
    pub fn get_command(&self) -> String {
        match &self {
            ATDomeCmd::MoveAz(az) => format!("{az} MV\r\n"),
            ATDomeCmd::CloseShutter => "SC".to_string(),
            ATDomeCmd::OpenShutter => "SO".to_string(),
            ATDomeCmd::StopMotion => "ST".to_string(),
            ATDomeCmd::HomeAzimuth => "HM".to_string(),
            ATDomeCmd::OpenShutterDropoutDoor => "DN".to_string(),
            ATDomeCmd::CloseShutterDropoutDoor => "UP".to_string(),
            ATDomeCmd::OpenShutterMainDoor => "OP".to_string(),
            ATDomeCmd::CloseShutterMainDoor => "CL".to_string(),
            ATDomeCmd::GetStatus => "+\r\n".to_string(),
            ATDomeCmd::Unknown => "".to_string(),
        }
    }

    pub fn from_str(atdome_cmd: &str) -> ATDomeCmd {
        ATDomeCmd::Unknown
    }
}

#[derive(Debug)]
pub enum ATDomeReply {
    None,
    Status(Status),
}

impl ATDomeReply {
    pub fn from_buffer(buffer: &[u8]) -> ATDomeReply {
        ATDomeReply::None
    }
}

#[derive(Debug)]
struct ATDomeModel {
    pub cmd_channel: mpsc::Sender<(ATDomeCmd, oneshot::Sender<ATDomeReply>)>,
    cmd_task: Option<task::JoinHandle<ATDomeResult<()>>>,
}

impl ATDomeModel {
    pub async fn create_and_start(
        host: &str,
        port: usize,
        cmd_channel_size: usize,
    ) -> ATDomeResult<ATDomeModel> {
        let (cmd_channel, mut cmd_receiver): (
            mpsc::Sender<(ATDomeCmd, oneshot::Sender<ATDomeReply>)>,
            mpsc::Receiver<(ATDomeCmd, oneshot::Sender<ATDomeReply>)>,
        ) = mpsc::channel(cmd_channel_size);

        let mut stream = TcpStream::connect(&format!("{host}:{port}")).await?;

        let cmd_task = Some(task::spawn(async move {
            let mut buffer = [0; 1024];

            // read welcome message and wait for the prompt character ">"
            loop {
                // read any message in the stream;
                let n_bytes = stream.read(&mut buffer).await?;

                if let Ok(reply) = str::from_utf8(&buffer[..n_bytes]) {
                    println!("Got {n_bytes} bytes:\n{}", reply);
                    if reply.contains(">") {
                        break;
                    }
                } else {
                    break;
                }
            }

            while let Some((atdome_cmd, atdome_reply_sender)) = cmd_receiver.recv().await {
                let status_parser = StatusParser::new()?;
                let command = atdome_cmd.get_command();
                println!("{atdome_cmd:?}::{command}");
                stream
                    .write_all(&atdome_cmd.get_command().into_bytes())
                    .await?;
                match atdome_cmd {
                    ATDomeCmd::GetStatus => {
                        println!("Handling status command");
                        let mut total_bytes = 0;
                        let mut status_str = String::with_capacity(1024);
                        loop {
                            // read any message in the stream;
                            let n_bytes = stream.read(&mut buffer).await?;
                            total_bytes = total_bytes + n_bytes;
                            println!("Got {n_bytes}: {buffer:?}");
                            if let Ok(reply) = str::from_utf8(&buffer[..n_bytes]) {
                                println!("Got {n_bytes} bytes:\n{}", reply);
                                status_str.push_str(reply);
                                if reply.contains(">") {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        println!("Total bytes read: {total_bytes}");
                        let status_vec: Vec<&str> = status_str.split("\n").collect();
                        match status_parser.make_status(&status_vec) {
                            Ok(status) => {
                                println!("Sending status: {status:?}");
                                if let Err(error) =
                                    atdome_reply_sender.send(ATDomeReply::Status(status))
                                {
                                    println!("Error sending reply: {error:?}");
                                }
                            }
                            Err(error) => println!("Error parsing status: {error}"),
                        }
                    }
                    _ => {
                        log::debug!("Waiting for prompt to return.");
                        loop {
                            let n_bytes = stream.read(&mut buffer).await?;
                            log::debug!("{buffer:?}");
                            if n_bytes == 0 {
                                break;
                            }
                        }
                        if let Err(error) = atdome_reply_sender.send(ATDomeReply::None) {
                            log::error!("Error sending reply: {error:?}");
                        }
                    }
                }
            }
            Ok(())
        }));

        Ok(ATDomeModel {
            cmd_channel,
            cmd_task,
        })
    }

    pub fn is_finished(&self) -> bool {
        if let Some(cmd_task) = &self.cmd_task {
            return cmd_task.is_finished();
        } else {
            return true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_atdome_model_get_status() {
        let atdome_model = ATDomeModel::create_and_start("127.0.0.1", 5001, 10)
            .await
            .unwrap();

        let (rx, tx) = oneshot::channel();

        let get_status = (ATDomeCmd::GetStatus, rx);

        atdome_model.cmd_channel.send(get_status).await.unwrap();

        if let ATDomeReply::Status(status) = tx.await.unwrap() {
            assert_eq!(status.az_pos, 285.0);
            assert_eq!(status.auto_shutdown_enabled, false);
            assert_eq!(status.az_home_switch, false);
            assert_eq!(status.az_pos, 285.0);
            assert_eq!(status.azimuth_move_timeout, 120.0);
            assert_eq!(status.cloud_sensor_enabled, true);
            assert_eq!(status.coast, 0.5);
            assert_eq!(status.door_move_timeout, 360.0);
            assert_eq!(status.dropout_door_encoder_closed, 5669776578);
            assert_eq!(status.dropout_door_encoder_opened, 5710996184);
            assert_eq!(status.dropout_door_pct, 0.0);
            assert_eq!(status.dropout_timer, 5.0);
            assert_eq!(status.encoder_counts, 111615089);
            assert_eq!(status.encoder_counts_per_360, 4018143232);
            assert_eq!(status.estop_active, false);
            assert_eq!(status.high_speed, 5.0);
            assert_eq!(status.home_azimuth, 10.0);
            assert_eq!(status.homed, false);
            assert_eq!(status.last_azimuth_goto, 285.0);
            assert_eq!(status.main_door_encoder_closed, 118449181478);
            assert_eq!(status.main_door_encoder_opened, 8287616388);
            assert_eq!(status.main_door_pct, 0.0);
            assert_eq!(status.move_code, 0);
            assert_eq!(status.rain_sensor_enabled, true);
            assert_eq!(status.reversal_delay, 4.0);
            assert_eq!(status.scb_link_ok, true);
            assert_eq!(status.sensor_code, 0);
            assert_eq!(status.tolerance, 1.0);
            assert_eq!(status.watchdog_timer, 600.0);
        } else {
            panic!("Expected to get Status.");
        }

        assert!(!atdome_model.is_finished());
    }
}
