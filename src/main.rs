use std::fs;
use std::path::Path;

use futures::{
    channel::mpsc::{channel, Receiver},
    SinkExt, StreamExt,
};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify::event::ModifyKind::Any;
use regex::Regex;

/// Async, futures channel based event watching
fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut args = std::env::args();
    let src = args
        .nth(1)
        .expect("Argument 1 needs to be a source path");
    println!("watching {}", src);
    let re = args.nth(0).expect("Argument 2 needs to be a regular expression");
    let regex = Regex::new(&re).expect("Argument 2 needs to be a valid regular expression");
    println!("matching {}", regex);
    let dst = args.nth(0).expect("Argument 3 needs to be a destination path");
    println!("copying to {}", dst);

    futures::executor::block_on(async {
        if let Err(e) = async_watch(src, regex, dst).await {
            println!("error: {:?}", e)
        }
    });
}

fn async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (mut tx, rx) = channel(1);

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let watcher = RecommendedWatcher::new(
        move |res| {
            futures::executor::block_on(async {
                tx.send(res).await.unwrap();
            })
        },
        Config::default(),
    )?;

    Ok((watcher, rx))
}

async fn async_watch<P: AsRef<Path>>(src: P, regex: Regex, target: P) -> notify::Result<()> {
    let (mut watcher, mut rx) = async_watcher()?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(src.as_ref(), RecursiveMode::Recursive)?;

    while let Some(res) = rx.next().await {
        match res {
            Ok(event) => {
                // log::info!("Change: {event:?}");
                if event.kind == EventKind::Modify(Any) {
                    log::debug!("Capture {event:?}");
                    let path = event.paths.get(0)
                        .unwrap()
                        .to_str()
                        .unwrap();
                    // test if path matches pattern
                    if regex.is_match(path) {
                        log::info!("Match {path}");
                        // copy file to destination
                        let src_path = Path::new(path);
                        let dst_path = Path::new(target.as_ref()).join(src_path.file_name().unwrap());
                        log::info!("Copy {} -> {}", src_path.display(), dst_path.display());
                        fs::copy(src_path, &dst_path).unwrap();
                    }
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }
    Ok(())
}
