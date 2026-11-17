//! App-managed popup input ownership, target/cpp-ref/src/app/MainWindowContextMenus.cpp:112-192 and KaRegionLocator.cpp:106-120.
use crate::app_state::StrataAppState;

impl StrataAppState {
    /// App-owned map menus; address forms and native egui popup IDs are outside this predicate (KaRegionLocator.cpp:176).
    pub(crate) fn app_popup_menu_open(&self) -> bool {
        self.ribbon.paleo_menu_open
            || self.ribbon.geology_menu_open
            || self.ribbon.dem_menu_open
            || self.ribbon.soil_menu_open
            || self.ribbon.cadastral_menu_open
            || self.ribbon.topo_menu_open
            || self.ribbon.old_map_menu_open
            || self.ribbon.more_menu_open
            || self.map_context_menu_open
            || self.layer_context_menu_open
            || self.ribbon.app_bar.region_popup_open
            || (self.ribbon.snap_assist_menu_open && self.draw_row_open())
            || self.theme_menu_open
            || self.scale_menu_open
    }
}
