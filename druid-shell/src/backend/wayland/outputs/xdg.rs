use wayland_client as wlc;
use wayland_protocols::unstable::xdg_output::v1::client::zxdg_output_manager_v1;

use super::super::error;
use super::super::outputs;

pub fn detect(
    registry: &wlc::GlobalManager,
) -> Result<calloop::channel::Channel<outputs::Event>, error::Error> {
    tracing::warn!("output detection not implemented using the xdg protocol");
    let (_outputsaddedtx, outputsaddedrx) = calloop::channel::channel::<outputs::Event>();
    let zxdg_output_manager = registry
        .instantiate_exact::<zxdg_output_manager_v1::ZxdgOutputManagerV1>(3)
        .map_err(|e| error::Error::global("zxdg_output_manager_v1", 3, e))?;

    zxdg_output_manager.quick_assign(|m, event, ctx| {
        tracing::info!("global zxdg output manager {:?} {:?} {:?}", m, ctx, event);
    });

    Ok(outputsaddedrx)
}
