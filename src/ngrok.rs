use reqwest::{Client, Url};
use serde::Deserialize;
use std::process::Stdio;
use std::sync::Mutex;
use tokio::process::Command;
use tokio::sync::oneshot::{channel, Sender};
use tokio::time::Duration;

pub struct Ngrok {
    kill_channel: Mutex<Option<Sender<()>>>,
}

impl Ngrok {
    pub fn new() -> Self {
        Ngrok {
            kill_channel: Mutex::new(None),
        }
    }

    pub fn start(
        &self,
        connection_type: &str,
        port: u32,
        domain: &Option<String>,
        kill_on_start: bool,
    ) -> Result<String, String> {
        if self.is_run() && kill_on_start {
            self.kill();
        }

        self._bump_version();

        let args = self._collect_args(connection_type, port, domain);

        let mut ngrok_cli_cmd = Command::new("ngrok");
        ngrok_cli_cmd
            .args(&args)
            .kill_on_drop(true)
            .stdout(Stdio::null());

        log::info!("Starting ngrok... {:#?}", ngrok_cli_cmd);

        if let Ok(mut child) = ngrok_cli_cmd.spawn() {
            let (send, recv) = channel::<()>();
            *self.kill_channel.lock().unwrap() = Some(send);

            tokio::spawn(async move {
                tokio::select! {
                    _ = child.wait() => {
                        log::info!("Ngrok was stopped by itself or by ctr+c sequence.");
                    }
                    _ = recv => {
                        child.kill().await.expect("ngrok kill failed");
                        log::info!("Ngrok was killed!");
                    }
                }
            });

            Ok(format!(
                "Start {} connection on {} port",
                connection_type, port
            ))
        } else {
            Err(format!(
                "Failed to start {} connection on {} port",
                connection_type, port
            ))
        }
    }

    pub fn kill(&self) {
        let mut sender = self.kill_channel.lock().unwrap();
        if let Some(channel) = sender.take() {
            if channel.send(()).is_err() {
                log::error!("Fail to send a kill signal to ngrok!");
            }
        }
    }

    pub fn is_run(&self) -> bool {
        self.kill_channel.lock().unwrap().is_some()
    }

    pub async fn fetch_url(&self) -> Result<Url, String> {
        if !self.is_run() {
            return Err("Ngrok was not started!".to_string());
        }

        let client = Client::builder()
            .timeout(Duration::from_millis(1000))
            .build()
            .expect("Expected no TSL errors.");

        let api_tunnels_url = "http://localhost:4040/api/tunnels";

        let api_response = client
            .get(api_tunnels_url)
            .header("Content-Type", "application/json")
            .send()
            .await;

        if let Err(api_response) = api_response {
            return Err(api_response.to_string());
        }

        let response_content_json = api_response.unwrap().text().await;
        if let Err(response_content_json) = response_content_json {
            return Err(response_content_json.to_string());
        }

        log::info!("Ngrok response {}", response_content_json.as_ref().unwrap());

        let ngrok_tunnels =
            serde_json::from_str::<NgrokApiTunnels>(&response_content_json.unwrap());

        log::info!("{:?}", ngrok_tunnels);

        match ngrok_tunnels {
            Ok(ngrok_tunnels) => {
                ngrok_tunnels
                    .tunnels
                    .get(0)
                    .ok_or_else(|| "Error: no tunnels were returned by Ngrok".into())
                    .and_then(|tun| {
                        Url::parse(&tun.public_url).map_or(
                            Err("Bad URL returned from API".into()),
                            Ok, //
                        )
                    })
            }
            Err(err) => {
                log::error!("Can't parse ngrok API response: {}", err);
                Err("Can't parse ngrok API response".into())
            }
        }
    }

    fn _bump_version(&self) {
        let _ = Command::new("ngrok").args(["-v"]).spawn();
    }

    fn _collect_args(
        &self,
        connection_type: &str,
        port: u32,
        domain: &Option<String>,
    ) -> Vec<String> {
        let mut args: Vec<String> = vec![connection_type.to_string()];

        if let Some(domain) = domain {
            if connection_type == "http" {
                args.push("--domain".to_string());
                args.push(domain.to_owned());
            }
        }

        args.push(port.to_string());

        args
    }
}

#[derive(Deserialize, Debug)]
struct NgrokApiTunnels {
    tunnels: Vec<NgrokTunnels>,
}

#[derive(Deserialize, Debug)]
struct NgrokTunnels {
    public_url: String,
}
