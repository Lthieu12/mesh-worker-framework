fn main() {
}

/*
use anyhow::Result;

fn main() -> Result<()> {
    MWF::init()?;

    MWF::register_worker_type::<DemoWorker>()?;

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(run())?;

    MWF::close()?;

    Ok(())
}

async fn run() -> Result<()> {
    let main_entity = EntityID::new_external("MAIN");

    let worker = MWFWrk::spawn_worker("DemoWorker", &main_entity)?;

    MWFWrk::init_worker(&worker, Bytes::new()).await?;

    // ...

    MWFWrk::close_worker(&worker, Bytes::new()).await?;

    Ok(())
}
*/