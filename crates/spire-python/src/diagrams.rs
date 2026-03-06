//! `DiagramSet` - Python wrapper around [`graph::TopologySet`].

use pyo3::prelude::*;

use spire_kernel::graph::{self, LoopOrder};

use crate::errors::to_py_err;
use crate::model::PyModel;
use crate::reaction::PyReaction;

/// A set of Feynman diagrams generated for a reaction.
///
/// ```python
/// from spire import Model, Reaction, DiagramSet
///
/// sm = Model.standard_model()
/// rxn = Reaction.create(sm, ["e-", "e+"], ["mu-", "mu+"], cms_energy=91.2)
/// diagrams = DiagramSet.generate(rxn, sm, max_loop_order=0)
/// print(diagrams.count)
/// ```
#[pyclass(name = "DiagramSet")]
#[derive(Debug, Clone)]
pub struct PyDiagramSet {
    pub(crate) inner: graph::TopologySet,
    json_cache: String,
}

#[pymethods]
impl PyDiagramSet {
    /// Generate Feynman diagrams for a validated reaction.
    ///
    /// Parameters
    /// ----------
    /// reaction : Reaction
    ///     A previously constructed and validated reaction.
    /// model : Model
    ///     The theoretical model providing vertex rules.
    /// max_loop_order : int, optional
    ///     Maximum loop order (0 = tree-level, 1 = one-loop, etc.).
    #[staticmethod]
    #[pyo3(signature = (reaction, model, max_loop_order=0))]
    fn generate(reaction: &PyReaction, model: &PyModel, max_loop_order: u32) -> PyResult<Self> {
        let order = match max_loop_order {
            0 => LoopOrder::Tree,
            1 => LoopOrder::OneLoop,
            2 => LoopOrder::TwoLoop,
            n => LoopOrder::NLoop(n),
        };

        let topologies =
            graph::generate_topologies(&reaction.inner, &model.inner, order).map_err(to_py_err)?;

        let json = serde_json::to_string(&topologies)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(Self {
            inner: topologies,
            json_cache: json,
        })
    }

    // ── Properties ───────────────────────────────────────────────────

    /// Number of diagrams generated.
    #[getter]
    fn count(&self) -> usize {
        self.inner.count
    }

    /// Serialised JSON of all diagrams.
    fn to_json(&self) -> String {
        self.json_cache.clone()
    }

    // ── Display ──────────────────────────────────────────────────────

    fn __repr__(&self) -> String {
        format!("DiagramSet(count={})", self.inner.count)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __len__(&self) -> usize {
        self.inner.count
    }

    /// Jupyter rich HTML representation.
    fn _repr_html_(&self) -> String {
        let mut html = format!(
            "<div style=\"font-family:'Segoe UI',sans-serif; font-size:13px;\">\
             <h4 style=\"margin:0 0 6px 0;\">📊 Feynman Diagrams</h4>\
             <p>Generated <b>{}</b> diagram(s)</p>",
            self.inner.count,
        );

        // Show a brief summary of each diagram
        for (i, diagram) in self.inner.diagrams.iter().enumerate() {
            let nodes = diagram.graph.node_count();
            let edges = diagram.graph.edge_count();
            html.push_str(&format!(
                "<p style=\"margin:1px 0; font-size:12px; font-family:monospace;\">\
                 Diagram #{}: {} nodes, {} edges</p>",
                i + 1,
                nodes,
                edges,
            ));
            if i >= 19 {
                html.push_str(&format!(
                    "<p style=\"color:#888;\">…and {} more</p>",
                    self.inner.count - 20
                ));
                break;
            }
        }

        html.push_str("</div>");
        html
    }
}
