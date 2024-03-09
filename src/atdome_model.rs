//! Provide an interface to the ATDome Controller.

use crate::{
    error::{ATDomeError, ATDomeResult},
    status::Status,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{mpsc, oneshot},
    task,
    time::{sleep, timeout, Duration},
};

#[derive(Debug)]
enum ATDomeCmd {
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
}

impl ATDomeCmd {
    pub fn get_command(&self) -> String {
        match &self {
            ATDomeCmd::MoveAz(az) => format!("{az} MV"),
            ATDomeCmd::CloseShutter => "SC".to_string(),
            ATDomeCmd::OpenShutter => "SO".to_string(),
            ATDomeCmd::StopMotion => "ST".to_string(),
            ATDomeCmd::HomeAzimuth => "HM".to_string(),
            ATDomeCmd::OpenShutterDropoutDoor => "DN".to_string(),
            ATDomeCmd::CloseShutterDropoutDoor => "UP".to_string(),
            ATDomeCmd::OpenShutterMainDoor => "OP".to_string(),
            ATDomeCmd::CloseShutterMainDoor => "CL".to_string(),
            ATDomeCmd::GetStatus => "+".to_string(),
        }
    }
}

#[derive(Debug)]
enum ATDomeReply {
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
            let mut buffer = [0; 512];

            // read any message in the stream;
            let n_bytes = stream.read(&mut buffer).await?;

            log::debug!("Got {n_bytes} read: {buffer:?}.");

            while let Some((atdome_cmd, atdome_reply_sender)) = cmd_receiver.recv().await {
                log::debug!("{atdome_cmd:?}");
                stream
                    .write_all(&atdome_cmd.get_command().into_bytes())
                    .await?;
                match atdome_cmd {
                    ATDomeCmd::GetStatus => {
                        log::debug!("Handling status command");
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
                    }
                }

                if let Err(error) = atdome_reply_sender.send(ATDomeReply::from_buffer(&buffer)) {
                    log::error!("Error sending reply: {error:?}");
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
    async fn test_atdome_model() {
        let atdome_model = ATDomeModel::create_and_start("127.0.0.1", 5001, 10)
            .await
            .unwrap();

        let (rx, tx) = oneshot::channel();

        let get_status = (ATDomeCmd::GetStatus, rx);

        atdome_model.cmd_channel.send(get_status).await.unwrap();

        if let ATDomeReply::Status(status) = tx.await.unwrap() {
            assert!(status.homed);
        } else {
            panic!("Expected to get Status.");
        }

        assert!(!atdome_model.is_finished());
    }
}
