//! Implement the ATDome CSC.
//!

use crate::error::{ATDomeError, ATDomeResult};
use std::collections::{HashMap, HashSet};

use apache_avro::{from_value, types::Value};

use tokio::{
    sync::{mpsc, watch},
    task,
    time::{sleep, timeout, Duration},
};

use handle_command::handle_command;
use salobj::{
    controller::Controller,
    csc::{
        base_csc::{BaseCSC, HEARTBEAT_TIME},
        test_csc::topics::{arrays::Arrays, scalars::Scalars, telemetry::TestTelemetry},
    },
    domain::Domain,
    error::errors::SalObjResult,
    generics::{
        disable::Disable, empty_topic::EmptyTopic, enable::Enable, exit_control::ExitControl,
        heartbeat::Heartbeat, standby::Standby, start::Start, summary_state::SummaryState,
    },
    sal_enums::State,
    sal_info::SalInfo,
    topics::{
        base_sal_topic::BaseSALTopic, controller_command::ControllerCommand,
        controller_command_ack::ControllerCommandAck, write_topic::WriteTopic,
    },
    utils::{command_ack::CommandAck, types::WriteTopicSet},
};

type CmdPayload = (CmdData, mpsc::Sender<CommandAck>);
type CommandAckResult = (CommandAck, mpsc::Sender<CommandAck>);

struct CmdData {
    pub name: String,
    pub data: Value,
}

#[derive(Default, Clone, Copy)]
enum ATDomeTelemetry {
    #[default]
    None,
}

#[derive(Default)]
struct TelemetryPayload {
    pub name: String,
    pub data: ATDomeTelemetry,
}

pub struct ATDome<'a> {
    summary_state: State,
    domain: Domain,
    controller: Controller<'a>,
    controller_command_ack: Option<ControllerCommandAck>,
    heartbeat_task: Option<task::JoinHandle<()>>,
    telemetry_loop_task: Option<task::JoinHandle<()>>,
    command_sender: mpsc::Sender<CmdPayload>,
    command_receiver: mpsc::Receiver<CmdPayload>,
    telemetry_sender: watch::Sender<TelemetryPayload>,
    telemetry_receiver: watch::Receiver<TelemetryPayload>,
}

impl<'a> ATDome<'a> {
    pub fn new() -> ATDomeResult<ATDome<'a>> {
        let mut domain = Domain::new();
        let controller = Controller::new(&mut domain, "ATDome", 0)?;
        let (command_sender, command_receiver): (
            mpsc::Sender<CmdPayload>,
            mpsc::Receiver<CmdPayload>,
        ) = mpsc::channel(32);

        let (telemetry_sender, telemetry_receiver): (
            watch::Sender<TelemetryPayload>,
            watch::Receiver<TelemetryPayload>,
        ) = watch::channel(TelemetryPayload::default());

        Ok(ATDome {
            summary_state: State::Standby,
            domain,
            controller,
            controller_command_ack: None,
            heartbeat_task: None,
            telemetry_loop_task: None,
            command_sender,
            command_receiver,
            telemetry_sender,
            telemetry_receiver,
        })
    }

    /// Start the CSC.
    ///
    /// This method should run only once after instantiating the CSC and will
    /// setup a series of background tasks that operates the CSC.
    pub async fn start(&mut self) -> ATDomeResult<()> {
        self.update_summary_state().await?;

        let sal_info = SalInfo::new("ATDome", 0)?;

        sal_info.register_schema().await;

        self.domain.register_topics(&sal_info.get_topics_name())?;

        let mut heartbeat_writer = WriteTopic::new("logevent_heartbeat", &sal_info, &self.domain);

        let heartbeat_task = task::spawn(async move {
            let origin = heartbeat_writer.get_origin();
            let identity = heartbeat_writer.get_identity();
            let sal_index = heartbeat_writer.get_index();
            loop {
                let seq_num = heartbeat_writer.get_seq_num();

                let heartbeat_topic = Heartbeat::default()
                    .with_timestamps()
                    .with_sal_index(sal_index)
                    .with_private_origin(origin)
                    .with_private_identity(&identity)
                    .with_private_seq_num(seq_num);
                let write_res = heartbeat_writer
                    .write_typed::<Heartbeat>(&heartbeat_topic)
                    .await;
                if write_res.is_err() {
                    log::error!("Failed to write heartbeat data {write_res:?}.");
                    break;
                }
                sleep(HEARTBEAT_TIME).await;
            }
        });

        self.heartbeat_task = Some(heartbeat_task);

        let controller_command_ack = ControllerCommandAck::start(&self.domain, &sal_info).await;

        for command in sal_info.get_command_names() {
            let controller_command_ack_sender = controller_command_ack.ack_sender.clone();
            log::debug!("Registering command {command}.");
            let command_sender = self.command_sender.clone();
            let mut controller_command =
                ControllerCommand::new(&command, &self.domain, &sal_info).unwrap();

            task::spawn(async move {
                loop {
                    if let Ok(command_data) = controller_command.process_command().await {
                        let ack_sender = controller_command_ack_sender.clone();
                        let _ = command_sender
                            .send((
                                CmdData {
                                    name: command.to_owned(),
                                    data: command_data,
                                },
                                ack_sender,
                            ))
                            .await;
                    }
                }
            });
        }

        self.controller_command_ack = Some(controller_command_ack);
        Ok(())
    }

    /// This method runs the control loop of the CSC.
    ///
    /// Once awaited the CSC will start to respond to commands.
    pub async fn run(&mut self) -> ATDomeResult<()> {
        while let Some((data, ack_channel)) = self.command_receiver.recv().await {
            handle_command!("start", "standby", "enable", "disable",);
        }
        Ok(())
    }

    /// Respond to the start command.
    ///
    /// This will transition the CSC from Standby to Disabled.
    async fn do_start(
        &mut self,
        data: &CmdData,
        ack_channel: mpsc::Sender<CommandAck>,
    ) -> ATDomeResult<CommandAckResult> {
        log::info!("do_start received {:?}", data.name);
        let start = from_value::<Start>(&data.data).unwrap();
        let current_state = self.get_current_state();
        if current_state != State::Standby {
            return Ok((
                CommandAck::make_failed(
                    start,
                    1,
                    &format!("Invalid state transition {current_state:?} -> Disable."),
                ),
                ack_channel,
            ));
        }
        let _ = self.configure(&start);

        let sal_info = SalInfo::new("Test", 0).unwrap();

        let mut telemetry_writers: WriteTopicSet = sal_info
            .get_telemetry_names()
            .into_iter()
            .map(|telemetry_name| {
                (
                    telemetry_name.to_owned(),
                    WriteTopic::new(&telemetry_name, &sal_info, &self.domain),
                )
            })
            .collect();

        let mut telemetry_received = self.telemetry_receiver.clone();

        let telemetry_loop_task = task::spawn(async move {
            log::debug!("Telemetry task starting");

            let mut telemetry_data: HashMap<String, ATDomeTelemetry> = HashMap::new();

            // HashMap::from([
            //     (
            //         "scalars".to_owned(),
            //         TestTelemetry::Scalars(Scalars::default()),
            //     ),
            //     (
            //         "arrays".to_owned(),
            //         TestTelemetry::Arrays(Arrays::default()),
            //     ),
            // ]);

            loop {
                let loop_time_task = task::spawn(async { sleep(Duration::from_secs(1)).await });

                if timeout(Duration::from_secs(1), telemetry_received.changed())
                    .await
                    .is_ok()
                {
                    let new_telemetry = telemetry_received.borrow();
                    log::debug!("Updating telemetry data for {}", new_telemetry.name);
                    *telemetry_data
                        .entry(new_telemetry.name.to_owned())
                        .or_insert(new_telemetry.data) = new_telemetry.data;
                } else {
                    log::trace!("Telemetry not updated.");
                }

                // for (telemetry_name, telemetry_writer) in telemetry_writers.iter_mut() {
                //     let name = telemetry_name.as_str();
                //     if let Some(telemetry_data_to_write) = telemetry_data.get_mut(name) {
                //         match telemetry_data_to_write {
                //             TestTelemetry::Scalars(scalar) => {
                //                 let _ = telemetry_writer.write_typed::<Scalars>(scalar).await;
                //             }
                //             TestTelemetry::Arrays(array) => {
                //                 let _ = telemetry_writer.write_typed::<Arrays>(array).await;
                //             }
                //             TestTelemetry::None => {}
                //         }
                //     }
                // }
                let _ = loop_time_task.await;
            }
        });
        self.telemetry_loop_task = Some(telemetry_loop_task);

        self.set_summary_state(State::Disabled);
        self.update_summary_state().await?;
        Ok((CommandAck::make_complete(start), ack_channel))
    }

    /// Respond to the disable command.
    ///
    /// This command will transition the CSC from Enabled to Disabled.
    async fn do_disable(
        &mut self,
        data: &CmdData,
        ack_channel: mpsc::Sender<CommandAck>,
    ) -> ATDomeResult<CommandAckResult> {
        log::info!("do_disabled received {:?}", data.name);
        let disable = from_value::<Disable>(&data.data).unwrap();
        let current_state = self.get_current_state();
        if current_state != State::Enabled {
            return Ok((
                CommandAck::make_failed(
                    disable,
                    1,
                    &format!("Invalid state transition {current_state:?} -> Disable."),
                ),
                ack_channel,
            ));
        }
        self.set_summary_state(State::Disabled);
        self.update_summary_state().await?;
        if let Some(telemetry_loop_task) = &self.telemetry_loop_task {
            log::debug!("Stopping telemetry task.");
            telemetry_loop_task.abort();
        }
        Ok((CommandAck::make_complete(disable), ack_channel))
    }

    async fn do_enable(
        &mut self,
        data: &CmdData,
        ack_channel: mpsc::Sender<CommandAck>,
    ) -> ATDomeResult<CommandAckResult> {
        log::info!("do_enable received {:?}", data.name);
        let enable = from_value::<Enable>(&data.data).unwrap();
        let current_state = self.get_current_state();
        if current_state != State::Disabled {
            return Ok((
                CommandAck::make_failed(
                    enable,
                    1,
                    &format!("Invalid state transition {current_state:?} -> Enabled."),
                ),
                ack_channel,
            ));
        }
        self.set_summary_state(State::Enabled);
        self.update_summary_state().await?;

        Ok((CommandAck::make_complete(enable), ack_channel))
    }

    /// Respond to the standby command.
    ///
    /// This command will transition the CSC from Fault or Disabled into
    /// Standby.
    async fn do_standby(
        &mut self,
        data: &CmdData,
        ack_channel: mpsc::Sender<CommandAck>,
    ) -> ATDomeResult<CommandAckResult> {
        log::info!("do_standby received {:?}", data.name);
        let standby = from_value::<Standby>(&data.data).unwrap();
        let current_state = self.get_current_state();

        if !HashSet::from([State::Fault, State::Disabled]).contains(&current_state) {
            return Ok((
                CommandAck::make_failed(
                    standby,
                    1,
                    &format!("Invalid state transition {current_state:?} -> Standby."),
                ),
                ack_channel,
            ));
        }
        self.set_summary_state(State::Standby);
        self.update_summary_state().await?;
        Ok((CommandAck::make_complete(standby), ack_channel))
    }

    /// Respond to the exitControl command.
    ///
    /// If the CSC is in Standby, this will terminate the CSC execution.
    async fn do_exit_control(
        &mut self,
        data: &CmdData,
        ack_channel: mpsc::Sender<CommandAck>,
    ) -> ATDomeResult<CommandAckResult> {
        let exit_control = from_value::<ExitControl>(&data.data).unwrap();
        let current_state = self.get_current_state();
        if current_state != State::Standby {
            return Ok((
                CommandAck::make_failed(
                    exit_control,
                    1,
                    &format!("Invalid state transition {current_state:?} -> Offline."),
                ),
                ack_channel,
            ));
        }
        self.set_summary_state(State::Offline);
        self.update_summary_state().await?;
        Ok((CommandAck::make_complete(exit_control), ack_channel))
    }

    /// Publish the current state of the component.
    async fn update_summary_state(&mut self) -> ATDomeResult<()> {
        let summary_state = self
            .controller
            .get_event_to_write::<SummaryState>("logevent_summaryState")?
            .with_summary_state(self.summary_state);

        if let Err(err) = self
            .controller
            .write_event("logevent_summaryState", &summary_state)
            .await
        {
            return Err(ATDomeError::new(&format!(
                "Failed to write summary state: {err:?}"
            )));
        }
        Ok(())
    }
}

impl<'a> BaseCSC for ATDome<'a> {
    fn get_current_state(&self) -> State {
        self.summary_state
    }

    fn set_summary_state(&mut self, new_state: State) {
        self.summary_state = new_state;
    }

    fn configure(&mut self, data: &Start) -> SalObjResult<()> {
        log::info!(
            "Received {} configuration override.",
            data.get_configuration_override()
        );
        Ok(())
    }
}
