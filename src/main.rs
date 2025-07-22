use std::str::FromStr;

use chrono::{DateTime, Datelike, TimeDelta, Timelike, Utc};
use clap::Parser;
use color_eyre::eyre::{self, Context, eyre};
use cron::Schedule;
use tracing::{error, info};
use url::Url;
use webhook::{client::WebhookClient, models::Message};

mod models;

use crate::models::{Asset, Attachment};

/// A tiny tool to push hourly wolf images to Discord.
#[derive(Parser)]
struct Args {
    /// The webhook URL.
    url: Url,
    /// The host URL.
    #[clap(default_value = "https://hourly.photo/u/wolves/")]
    host: Url,
    /// Run the hook immediately and exit.
    #[clap(long)]
    now: bool,
    /// The schedule to run against.
    #[clap(short, long, default_value = "0 30 * * * * *")]
    schedule: String,
}

fn format_path(date: DateTime<Utc>) -> String {
    format!(
        "p/{:0>2}{:0>2}/{:0>2}/{:0>2}",
        date.year() - 2000,
        date.month(),
        date.day(),
        date.hour(),
    )
}

#[tracing::instrument]
fn get_asset_url(host: &Url, date: DateTime<Utc>) -> eyre::Result<Url> {
    Ok(host.join(&format_path(date))?)
}

#[tracing::instrument]
async fn get_asset(host: &Url, date: DateTime<Utc>) -> eyre::Result<Asset> {
    let url = get_asset_url(host, date)?;
    info!("Fetching asset from {}", url);
    reqwest::get(url)
        .await
        .context("request failed")?
        .error_for_status()?
        .json::<Asset>()
        .await
        .context("parsing failed")
}

#[tracing::instrument]
async fn get_first_attachment(host: &Url, date: DateTime<Utc>) -> eyre::Result<Attachment> {
    get_asset(host, date)
        .await
        .context("fetching asset failed")?
        .attachment
        .into_iter()
        .next()
        .ok_or(eyre!("missing attachment"))
}

#[tracing::instrument]
fn build_message(
    host: &Url,
    ev_time: DateTime<Utc>,
    attachment: Attachment,
) -> eyre::Result<Message> {
    let url = get_asset_url(host, ev_time)?;
    let mut msg = Message::new();
    msg.embed(|embed| {
        embed
            .image(&attachment.url)
            .timestamp(ev_time.to_rfc3339().as_str())
            .description(&format!(
                "[LINK]({}) · [PERMALINK]({})",
                url.as_str(),
                attachment.url
            ))
            .footer(
                "Made with <3 by @kaylendog · Provided by hourly.photo",
                None,
            )
    });
    Ok(msg)
}

#[tracing::instrument(skip(client))]
async fn dispatch_message(
    client: &WebhookClient,
    host: &Url,
    ev_time: DateTime<Utc>,
) -> eyre::Result<()> {
    // delay to a second after the time to ensure clock looks nice
    loop {
        let delta = ev_time - Utc::now();
        if delta < TimeDelta::zero() {
            break;
        }
        info!(
            "Next event scheduled for {} - sleeping for {}",
            ev_time, delta
        );
        tokio::time::sleep(delta.to_std().context("negative time duration")?).await;
    }
    // fetch attachment
    let attachment = get_first_attachment(host, ev_time).await?;

    // construct client and send
    info!("Dispatching event to Discord");
    let msg = build_message(host, ev_time, attachment)?;
    client.send_message(&msg).await.map_err(|err| eyre!(err))?;

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    let Args {
        url,
        host,
        now,
        schedule,
    } = Args::parse();

    // setup logging
    tracing_subscriber::fmt().init();
    color_eyre::install()?;

    let client = WebhookClient::new(url.as_str());

    // dispatch immediately if --now specified
    if now {
        dispatch_message(&client, &host, Utc::now()).await?;
        return Ok(());
    }

    // loop over upcoming events and dispatch
    let schedule = Schedule::from_str(&schedule).unwrap();
    for ev_time in schedule.upcoming(Utc) {
        dispatch_message(&client, &host, ev_time).await?;
    }

    Ok(())
}
