use serde::{Deserialize, Serialize};
use sky130pdk::Sky130Pdk;
use spice::Spice;
use std::path::PathBuf;
use substrate::arcstr::ArcStr;
use substrate::block::Block;
use substrate::context::{Context, PdkContext};
use substrate::io::schematic::HardwareType;
use substrate::io::{Array, InOut, Input, Io, Output, Signal};
use substrate::schematic::{CellBuilder, ExportsNestedData, Schematic};

#[derive(Io, Clone, Debug)]
pub struct SramIo {
    addr: Input<Array<Signal>>,
    din: Input<Array<Signal>>,
    we: Input<Signal>,
    wmask: Input<Array<Signal>>,
    clk: Input<Signal>,
    dout: Output<Array<Signal>>,
    vdd: InOut<Signal>,
    vss: InOut<Signal>,
}
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct SramMacro {
    width: usize,
    depth: usize,
    mask_width: usize,
    mux_ratio: usize,
    netlist_path: PathBuf,
}

impl SramMacro {
    /// The width of the address port, in bits.
    pub fn addr_width(&self) -> usize {
        self.depth.ilog2() as usize
    }
}

impl Block for SramMacro {
    type Io = SramIo;

    fn id() -> ArcStr {
        arcstr::literal!("sram_macro")
    }

    fn io(&self) -> Self::Io {
        SramIo {
            addr: Input(Array::new(self.addr_width(), Signal)),
            din: Input(Array::new(self.width, Signal)),
            we: Input(Signal),
            wmask: Input(Array::new(self.mask_width, Signal)),
            clk: Input(Signal),
            dout: Output(Array::new(self.width, Signal)),
            vdd: InOut(Signal),
            vss: InOut(Signal),
        }
    }
}

impl ExportsNestedData for SramMacro {
    type NestedData = ();
}

impl Schematic<Spice> for SramMacro {
    fn schematic(
        &self,
        io: &<<Self as Block>::Io as HardwareType>::Bundle,
        cell: &mut CellBuilder<Spice>,
    ) -> substrate::error::Result<Self::NestedData> {
        let mut scir = Spice::scir_cell_from_file(
            &self.netlist_path,
            &format!(
                "sram22_{}x{}m{}w{}",
                self.depth,
                self.width,
                self.mux_ratio,
                self.width / self.mask_width
            ),
        );

        for i in 0..self.addr_width() {
            scir.connect(&format!("ADDR[{i}]"), io.addr[i]);
        }
        scir.connect("WE", io.we);
        for i in 0..self.mask_width {
            scir.connect(&format!("WMASK[{i}]"), io.wmask[i]);
        }
        for i in 0..self.width {
            scir.connect(&format!("DIN[{i}]"), io.din[i]);
            scir.connect(&format!("DOUT[{i}]"), io.dout[i]);
        }
        scir.connect("VSS", io.vss);
        scir.connect("VDD", io.vdd);
        scir.connect("CLK", io.clk);

        cell.set_scir(scir);
        Ok(())
    }
}

/// Create a new Substrate context for the SKY130 commercial PDK.
///
/// Sets the PDK root to the value of the `SKY130_COMMERCIAL_PDK_ROOT`
/// environment variable and installs Spectre with default configuration.
///
/// # Panics
///
/// Panics if the `SKY130_COMMERCIAL_PDK_ROOT` environment variable is not set,
/// or if the value of that variable is not a valid UTF-8 string.
pub fn sky130_commercial_ctx() -> PdkContext<Sky130Pdk> {
    // Open PDK needed for standard cells.
    let open_pdk_root = std::env::var("SKY130_OPEN_PDK_ROOT")
        .expect("the SKY130_OPEN_PDK_ROOT environment variable must be set");
    let commercial_pdk_root = std::env::var("SKY130_COMMERCIAL_PDK_ROOT")
        .expect("the SKY130_COMMERCIAL_PDK_ROOT environment variable must be set");
    Context::builder()
        .install(spectre::Spectre::default())
        .install(Sky130Pdk::new(open_pdk_root, commercial_pdk_root))
        .build()
        .with_pdk()
}

#[cfg(test)]
mod tests {
    use crate::*;
    use substrate::schematic::netlist::ConvertibleNetlister;

    fn sram_512x64m4w8_pex() -> SramMacro {
        SramMacro {
            width: 64,
            depth: 512,
            mask_width: 8,
            mux_ratio: 4,
            netlist_path: PathBuf::from("/tools/C/rahulkumar/personal/sram22_sky130_macros/sram22_512x64m4w8/pex/schematic.pex.spice"),
        }
    }

    #[test]
    fn export_sram_macro() {
        let sram = sram_512x64m4w8_pex();
        let ctx = sky130_commercial_ctx();
        let lib = ctx.export_scir::<Spice, _>(sram).unwrap();

        Spice
            .write_scir_netlist_to_file(&lib.scir, "build/schematic.spice", Default::default())
            .expect("failed to write schematic");
    }
}
