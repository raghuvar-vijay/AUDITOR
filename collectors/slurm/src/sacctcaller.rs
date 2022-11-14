// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::{collections::HashMap, fmt, time::Duration};

use auditor::{
    constants::FORBIDDEN_CHARACTERS,
    domain::{Component, RecordAdd, Score},
};
use color_eyre::eyre::Result;
use fake::{Fake, Faker};
use regex::Regex;
use tokio::sync::mpsc;

use crate::{
    configuration::{AllowedTypes, Settings},
    shutdown::Shutdown,
    CONFIG,
};

type Job = HashMap<String, AllowedTypes>;

#[tracing::instrument(
    name = "Starting SacctCaller",
    skip(tx, _shutdown_notifier, shutdown, hold_till_shutdown)
)]
pub(crate) async fn run_sacct_monitor(
    frequency: Duration,
    tx: mpsc::Sender<RecordAdd>,
    _shutdown_notifier: mpsc::UnboundedSender<()>,
    mut shutdown: Shutdown,
    hold_till_shutdown: mpsc::Sender<()>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(frequency);
        loop {
            interval.tick().await;
            tokio::select! {
                records = get_job_info() => {
                    match records {
                        Ok(records) => place_records_on_queue(records, &tx).await,
                        Err(e) => {
                            tracing::error!("something went wrong: {:?}", e);
                            continue
                        }
                    };
                },
                _ = shutdown.recv() => {
                    tracing::info!("SacctCaller received shutdown signal. Shutting down.");
                    // shutdown properly
                    drop(hold_till_shutdown);
                    break
                },
            }
        }
    });
}

#[tracing::instrument(name = "Placing records on queue", level = "debug", skip(records, tx))]
async fn place_records_on_queue(records: Vec<RecordAdd>, tx: &mpsc::Sender<RecordAdd>) {
    for record in records {
        let record_id = record.record_id.clone();
        if let Err(e) = tx.send(record).await {
            tracing::error!("Could not send record {:?} to queue: {:?}", record_id, e);
        }
    }
}

#[tracing::instrument(name = "Calling sacct and parsing output")]
async fn get_job_info() -> Result<Vec<RecordAdd>> {
    // let job_id = 123;
    // let record = RecordAdd::new(
    //     format!("{}-{}", make_string_valid(&CONFIG.record_prefix), job_id),
    //     make_string_valid(&CONFIG.site_id),
    //     make_string_valid(&job["User"].extract_string()?),
    //     make_string_valid(&job["Group"].extract_string()?),
    //     construct_components(&CONFIG, &job),
    //     job["Start"].extract_datetime()?,
    // )
    // .expect("Could not construct record")
    // .with_stop_time(job["End"].extract_datetime()?);
    tokio::time::sleep(Duration::from_secs(5)).await;
    let record: auditor::domain::RecordTest = Faker.fake();

    Ok(vec![record.try_into().unwrap()])
}

#[tracing::instrument(name = "Remove forbidden characters from string", level = "debug")]
fn make_string_valid<T: AsRef<str> + fmt::Debug>(input: T) -> String {
    input.as_ref().replace(&FORBIDDEN_CHARACTERS[..], "")
}

#[tracing::instrument(
    name = "Construct components from job info and configuration",
    level = "debug"
)]
fn construct_components(config: &Settings, job: &Job) -> Vec<Component> {
    config
        .components
        .iter()
        .cloned()
        .filter(|c| {
            c.only_if.is_none() || {
                let only_if = c.only_if.as_ref().unwrap();
                let re = Regex::new(&only_if.matches)
                    .unwrap_or_else(|_| panic!("Invalid regex expression: {}", &only_if.matches));
                re.is_match(&job[&only_if.key].extract_string().unwrap_or_else(|_| {
                    panic!("Key is expectedto be a string: {:?}", job[&only_if.key])
                }))
            }
        })
        .map(|c| {
            Component::new(
                make_string_valid(c.name),
                job[&c.key].extract_i64().unwrap_or_else(|_| {
                    panic!(
                        "Cannot parse key {} (value: {:?}) into i64.",
                        c.key, job[&c.key]
                    )
                }),
            )
            .expect("Cannot construct component. Please check your configuration!")
            .with_scores(
                c.scores
                    .iter()
                    .filter(|s| {
                        s.only_if.is_none() || {
                            let only_if = s.only_if.as_ref().unwrap();
                            let re = Regex::new(&only_if.matches).unwrap_or_else(|_| {
                                panic!("Invalid regex expression: {}", &only_if.matches)
                            });
                            re.is_match(
                                &job[&only_if.key]
                                    .extract_string()
                                    .unwrap_or_else(|_| panic!("Error extracting string.")),
                            )
                        }
                    })
                    .map(|s| {
                        Score::new(s.name.clone(), s.factor)
                            .unwrap_or_else(|_| panic!("Cannot construct score from {:?}", s))
                    })
                    .collect(),
            )
        })
        .collect()
}
// let cmd_out = Command::new("/usr/bin/sacct")
//        .arg("-a")
//        .arg("-j")
//        .arg(job_id.to_string())
//        .arg("--format")
//        .arg(keys.iter().map(|k| k.0.clone()).join(","))
//        .arg("--noconvert")
//        .arg("--noheader")
//        .arg("-P")
//        .output()
//        .await?
//        .stdout;
// #[tracing::instrument(name = "Getting Slurm job info via scontrol")]
// fn get_slurm_job_info(job_id: u64) -> Result<Job, Error> {
//     Ok(std::str::from_utf8(
//         &Command::new("/usr/bin/scontrol")
//             .arg("show")
//             .arg("job")
//             .arg(job_id.to_string())
//             .arg("--details")
//             .output()?
//             .stdout,
//     )?
//     .split_whitespace()
//     .filter_map(|s| {
//         if let Some((k, v)) = s.split_once('=') {
//             Some((k.to_string(), v.to_string()))
//         } else {
//             None
//         }
//     })
//     .collect())
// }
