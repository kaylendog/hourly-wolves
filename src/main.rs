use std::str::FromStr;

use anyhow::{Context, anyhow};
use chrono::{DateTime, Datelike, Timelike, Utc};
use clap::Parser;
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

fn get_asset_url(host: &Url, date: DateTime<Utc>) -> anyhow::Result<Url> {
    Ok(host.join(&format_path(date))?)
}

async fn get_asset(host: &Url, date: DateTime<Utc>) -> anyhow::Result<Asset> {
    let url = get_asset_url(host, date)?;
    Ok(reqwest::get(url)
        .await
        .context("request failed")?
        .error_for_status()?
        .json::<Asset>()
        .await
        .context("parsing failed")?)
}

async fn get_first_attachment(host: &Url, date: DateTime<Utc>) -> anyhow::Result<Attachment> {
    get_asset(host, date)
        .await
        .context("fetching asset failed")?
        .attachment
        .into_iter()
        .next()
        .ok_or(anyhow!("missing attachment"))
}

fn build_message(
    host: &Url,
    ev_time: DateTime<Utc>,
    attachment: Attachment,
) -> anyhow::Result<Message> {
    let url = get_asset_url(&host, ev_time)?;
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

async fn dispatch_message(
    client: &WebhookClient,
    host: &Url,
    ev_time: DateTime<Utc>,
) -> anyhow::Result<()> {
    // delay to a second after the time to ensure clock looks nice
    loop {
        let delta = ev_time - Utc::now();
        if delta.num_milliseconds() < 0 {
            break;
        }
        info!(
            "Next event scheduled for {} - sleeping for {}",
            ev_time, delta
        );
        tokio::time::sleep(delta.to_std().context("negative time duration")?).await;
    }
    // fetch attachment
    let attachment = get_first_attachment(&host, ev_time).await?;

    // construct client and send
    info!("Dispatching event to Discord");
    let msg = build_message(&host, ev_time, attachment)?;
    client
        .send_message(&msg)
        .await
        .map_err(|err| anyhow!(err))?;

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let Args { url, host, .. } = Args::parse();
    tracing_subscriber::fmt().init();

    // setup client and schedule
    let client = WebhookClient::new(url.as_str());
    let schedule = Schedule::from_str("0 30 * * * * *").unwrap();

    // loop over upcoming events and dispatch
    for ev_time in schedule.upcoming(Utc) {
        if let Err(e) = dispatch_message(&client, &host, ev_time).await {
            error!("Error encountered during dispatch: {}", e);
        }
    }
}
