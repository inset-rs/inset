//! One window's valo surface: rendering a picture into a drawable and presenting it.
//!
//! Acquiring the drawable is where a frame is refused — the surface is what knows whether
//! the window can take one — so a draw hands the refusal back and leaves what to do about
//! it to the view (`view.rs`).

use inset_embedder::Picture;

use crate::os;

pub(crate) struct WinitSurface {
    /// `None` while the system holds the window's surface, between winit's `Suspended`
    /// and its next `Resumed`: every draw is refused as lost until it is made again.
    pub(crate) surface: Option<valo::Surface>,
    pub(crate) context: valo::Context,
    /// Whether the surface presents through the window's Core Animation transaction now.
    /// Off in the ordinary run of frames, since it makes every present wait to be
    /// scheduled; on for the frame drawn inside a resize, so the frame and the geometry
    /// it was laid out for are committed together. gpui does the same for its synchronous
    /// draws.
    pub(crate) in_transaction: bool,
}

impl WinitSurface {
    /// Renders `picture` into the surface's next drawable and presents it, cleared to
    /// `clear`. `Err` is the surface refusing a drawable.
    pub(crate) fn draw(
        &mut self,
        picture: &Picture,
        clear: valo::Color,
        in_transaction: bool,
    ) -> Result<(), valo::Refused> {
        self.present_in_transaction(in_transaction);
        let Some(surface) = &mut self.surface else {
            return Err(valo::Refused::Lost);
        };
        let surface_frame = surface.acquire()?;
        self.context
            .render(picture, &surface_frame.target(Some(clear)));
        self.context.present(surface_frame);
        Ok(())
    }

    fn present_in_transaction(&mut self, wanted: bool) {
        if self.in_transaction != wanted
            && let Some(surface) = &mut self.surface
        {
            os::present_in_transaction(surface, wanted);
            self.in_transaction = wanted;
        }
    }
}
