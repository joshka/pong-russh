use crate::tui;
use color_eyre::{config::HookBuilder, eyre};
use std::panic;

/// This replaces the standard color_eyre panic and error hooks with hooks that
/// restore the terminal before printing the panic or error.
pub fn install_hooks() -> color_eyre::Result<()> {
    let (panic_hook, eyre_hook) = HookBuilder::default().into_hooks();
    install_panic_hook(panic_hook);
    install_eyre_hook(eyre_hook)?;
    Ok(())
}

fn install_eyre_hook(eyre_hook: color_eyre::config::EyreHook) -> Result<(), eyre::Error> {
    let eyre_hook = eyre_hook.into_eyre_hook();
    eyre::set_hook(Box::new(
        move |error: &(dyn std::error::Error + 'static)| {
            tui::restore().unwrap();
            eyre_hook(error)
        },
    ))?;
    Ok(())
}

fn install_panic_hook(panic_hook: color_eyre::config::PanicHook) {
    // convert from a color_eyre PanicHook to a standard panic hook
    let panic_hook = panic_hook.into_panic_hook();
    panic::set_hook(Box::new(move |panic_info| {
        tui::restore().unwrap();
        panic_hook(panic_info);
    }));
}
