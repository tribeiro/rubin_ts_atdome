//! MTDome mock controller.

use crate::atdome_model::ATDomeReply;
use crate::error::ATDomeError;
use crate::move_code::MoveCode;
use crate::{
    atdome_cmd_regex::ATDomeCmdRegex, atdome_model::ATDomeCmd, error::ATDomeResult, status::Status,
};
use std::str;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, Duration};
use tokio::{net::TcpListener, task};

struct MockControllerCmd {
    pub atdome_cmd: ATDomeCmd,
    pub tx: oneshot::Sender<ATDomeReply>,
}

pub async fn run_mock_controller(port: usize) -> ATDomeResult<()> {
    let listener = TcpListener::bind(&format!("127.0.0.1:{port}")).await?;
    let (tx, mut rx) = mpsc::channel::<MockControllerCmd>(100);

    tokio::spawn(async move {
        let mut status = Status::default();
        status.scb_link_ok = true;
        status.high_speed = 6.0;
        status.main_door_encoder_closed = 118449181478;
        status.main_door_encoder_opened = 8287616388;
        status.dropout_door_encoder_closed = 5669776578;
        status.dropout_door_encoder_opened = 5710996184;

        // How much the dome can move per cycle.
        // 1 cycle is equal to 50 milliseconds.
        // This is equivalent to 6 deg/s.
        let delta_az_per_cycle = 0.12;
        // How much the main door can move per cycle (in %).
        let main_door_move_speed = 5;
        // How much the dropout door can move per cycle (in %).
        let dropout_door_move_speed = 2.5;

        loop {
            match rx.try_recv() {
                Ok(cmd) => {
                    let _ = match cmd.atdome_cmd {
                        ATDomeCmd::GetStatus => cmd.tx.send(ATDomeReply::Status(status)),
                        ATDomeCmd::MoveAz(new_az) => {
                            status.last_azimuth_goto = new_az;
                            cmd.tx.send(ATDomeReply::None)
                        }
                        ATDomeCmd::StopMotion => {
                            if status.last_azimuth_goto != status.az_pos {
                                // This makes sure the dome "stops moving"
                                // if it was moving before. It is just a way
                                // to emulate the operation and does not have
                                // any physics to it.
                                status.last_azimuth_goto = status.az_pos;
                                if status.move_code & MoveCode::AzimuthPositive.byte_value() > 0 {
                                    status.move_code =
                                        status.move_code ^ MoveCode::AzimuthPositive.byte_value();
                                } else if status.move_code & MoveCode::AzimuthNegative.byte_value()
                                    > 0
                                {
                                    status.move_code =
                                        status.move_code ^ MoveCode::AzimuthNegative.byte_value();
                                }
                            }
                            cmd.tx.send(ATDomeReply::None)
                        }
                        ATDomeCmd::OpenShutter => cmd.tx.send(ATDomeReply::None),
                        ATDomeCmd::Unknown => cmd.tx.send(ATDomeReply::None),
                        ATDomeCmd::HomeAzimuth => cmd.tx.send(ATDomeReply::None),
                        ATDomeCmd::CloseShutter => cmd.tx.send(ATDomeReply::None),
                        ATDomeCmd::OpenShutterMainDoor => cmd.tx.send(ATDomeReply::None),
                        ATDomeCmd::CloseShutterMainDoor => cmd.tx.send(ATDomeReply::None),
                        ATDomeCmd::OpenShutterDropoutDoor => cmd.tx.send(ATDomeReply::None),
                        ATDomeCmd::CloseShutterDropoutDoor => cmd.tx.send(ATDomeReply::None),
                    };
                }
                Err(err) => match err {
                    TryRecvError::Empty => {}
                    TryRecvError::Disconnected => break,
                },
            };
            // TODO Emulate behaviour here
            if status.az_pos != status.last_azimuth_goto
                && (status.move_code == 0
                    || status.move_code == MoveCode::AzimuthPositive.byte_value()
                    || status.move_code == MoveCode::AzimuthNegative.byte_value())
            {
                let delta_az = status.last_azimuth_goto - status.az_pos;
                if delta_az.abs() > delta_az_per_cycle {
                    if delta_az > 0.0 {
                        if status.move_code == 0 {
                            status.move_code =
                                status.move_code ^ MoveCode::AzimuthPositive.byte_value();
                        }
                        status.az_pos += delta_az_per_cycle;
                    } else {
                        if status.move_code == 0 {
                            status.move_code =
                                status.move_code ^ MoveCode::AzimuthNegative.byte_value();
                        }
                        status.az_pos -= delta_az_per_cycle;
                    }
                } else {
                    if status.move_code & MoveCode::AzimuthPositive.byte_value() > 0 {
                        status.move_code =
                            status.move_code ^ MoveCode::AzimuthPositive.byte_value();
                    } else if status.move_code & MoveCode::AzimuthNegative.byte_value() > 0 {
                        status.move_code =
                            status.move_code ^ MoveCode::AzimuthNegative.byte_value();
                    }
                    status.move_code = 0;
                    status.az_pos = status.last_azimuth_goto;
                }
            }
            // Then sleep for 50 milliseconds
            sleep(Duration::from_millis(50)).await;
        }
    });

    let atdome_cmd_regex = ATDomeCmdRegex::new();

    loop {
        let (mut socket, _) = listener.accept().await?;

        let mut buf = vec![0; 1024];

        // write prompt
        socket.write_all(b">").await?;

        loop {
            match socket.read(&mut buf).await {
                // Return value of `Ok(0)` signifies that the remote has
                // closed
                Ok(0) => break,
                Ok(n) => {
                    if let Ok(cmd) = str::from_utf8(&buf[..n]) {
                        let cmd_trimmed = cmd.trim_end_matches("\r\n");
                        let atdome_cmd = atdome_cmd_regex.into_atdome_cmd(cmd_trimmed);
                        if matches!(atdome_cmd, ATDomeCmd::Unknown) {
                            println!("Unknown dome command: {cmd_trimmed}.");
                        } else {
                            let (mock_controller_tx, mock_controller_rx) = oneshot::channel();
                            let mock_controller_cmd = MockControllerCmd {
                                atdome_cmd,
                                tx: mock_controller_tx,
                            };
                            let _ = tx.send(mock_controller_cmd).await;
                            if let Ok(mock_controller_response) = mock_controller_rx.await {
                                if let ATDomeReply::Status(status) = mock_controller_response {
                                    let _ =
                                        socket.write_all(&status.as_string().into_bytes()).await;
                                }
                            } else {
                                println!(
                                    "Internal error when requesting response from controller loop."
                                );
                                break;
                            }
                        }
                    }
                    if socket.write_all(b">").await.is_err() {
                        // Unexpected socket error. There isn't much we can
                        // do here so just stop processing.
                        return Ok(());
                    }
                }
                Err(error) => {
                    // Unexpected socket error. There isn't much we can do
                    // here so just stop processing.
                    return Err(ATDomeError::new(&error.to_string()));
                }
            }
        }
    }
}
