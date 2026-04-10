use async_trait::async_trait;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::{
    task::JoinSet,
    time::{Duration, sleep},
};

#[async_trait]
pub trait Request: Clone + Send + 'static {
    type Response: Send + Default + Clone;

    async fn send(
        &self,
        client: &mut t_invest_api_rust::Client,
    ) -> Result<tonic::Response<Self::Response>, tonic::Status>;
}

async fn send_request_until_success<T: Request>(
    request: T,
    mut client: t_invest_api_rust::Client,
) -> tonic::Response<T::Response> {
    loop {
        match request.send(&mut client).await {
            Ok(response) => {
                break response;
            }
            Err(status) => {
                assert_eq!(status.code(), tonic::Code::ResourceExhausted);
                let reset_secs: u64 = status
                    .metadata()
                    .get("x-ratelimit-reset")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .parse()
                    .unwrap();
                let wait_secs = reset_secs;
                //dbg!(wait_secs);
                sleep(Duration::from_secs(wait_secs)).await;
            }
        }
    }
}

pub async fn send_requests<T: Request>(
    requests: &[T],
    clients: &mut [t_invest_api_rust::Client],
    max_concurrent_requests: usize,
) -> Vec<T::Response> {
    let progress_bar = ProgressBar::new(requests.len() as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{wide_bar} {pos}/{len} ({percent}%) ETA: {eta}")
            .unwrap(),
    );

    let request_count = requests.len();
    let mut set = JoinSet::new();
    let mut next_request_index = 0;
    while next_request_index < max_concurrent_requests && next_request_index < request_count {
        let request = requests[next_request_index].clone();
        let client = clients[next_request_index % clients.len()].clone();
        set.spawn(async move { (next_request_index, send_request_until_success(request, client).await) });
        next_request_index += 1;
    }
    let mut responses = vec![T::Response::default(); request_count];
    while let Some(res) = set.join_next().await {
        let (request_index, response) = res.unwrap();
        let (_metadata, response, _extension) = response.into_parts();

        responses[request_index] = response;

        /*
        let remaining = metadata.get("x-ratelimit-remaining").unwrap().to_str().unwrap();
        let reset = metadata.get("x-ratelimit-reset").unwrap().to_str().unwrap();
        dbg!(remaining);
        dbg!(reset);
        dbg!();
        */

        progress_bar.inc(1);

        if next_request_index < request_count {
            let request = requests[next_request_index].clone();
            let client = clients[next_request_index % clients.len()].clone();
            set.spawn(async move { (next_request_index, send_request_until_success(request, client).await) });
            next_request_index += 1;
        }
    }

    responses
}
