use model_merger_engine::{
    NoopObserver,
    ammunition::{self, FillRequest},
};
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() < 3 {
        return Err("Usage: fill_magazine WEAPON.cast AMMO.cast OUTPUT.cast [MAGAZINE ...]".into());
    }
    let weapon = PathBuf::from(&args[0]);
    let analysis = ammunition::inspect(&weapon, &NoopObserver)?;
    println!("{analysis:#?}");
    let magazines = if args.len() > 3 {
        args[3..]
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect()
    } else {
        analysis
            .magazines
            .first()
            .map(|m| vec![m.name.clone()])
            .unwrap_or_default()
    };
    let result = ammunition::fill(
        FillRequest {
            weapon,
            ammunition: PathBuf::from(&args[1]),
            output: PathBuf::from(&args[2]),
            magazines,
        },
        &NoopObserver,
    )?;
    println!("{result:#?}");
    Ok(())
}
