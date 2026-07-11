// Copyright (c) 2026 かぜまる (Kazemaru) / Antigravity AI.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// ---
// 🐾 T.A.N.U.K.I. Project - Flat-AST Context Architecture Layer
// "バグは剪定されるべき枝葉、ハードコードは偽りの果実です。"

use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use tanuki_core::{FlatAST, MmapMemoryManager};

#[pyclass]
struct PyTanukiEngine {
    manager: MmapMemoryManager,
}

#[pymethods]
impl PyTanukiEngine {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let manager = MmapMemoryManager::new(path).map_err(|e| {
            PyErr::new::<PyException, _>(format!("Failed to load memory file: {}", e))
        })?;
        Ok(Self { manager })
    }

    fn update_mapping(&self, path: &str) -> PyResult<()> {
        self.manager.update_mapping(path).map_err(|e| {
            PyErr::new::<PyException, _>(format!("Failed to update mapping: {}", e))
        })?;
        Ok(())
    }

    fn search(&self, query_vector: Vec<f32>, top_k: usize) -> PyResult<Vec<(u64, f32)>> {
        if query_vector.len() != 768 {
            return Err(PyErr::new::<PyException, _>(
                "Query vector must be exactly 768 dimensions",
            ));
        }
        let mut query_array = [0.0f32; 768];
        query_array.copy_from_slice(&query_vector[0..768]);

        let results = self
            .manager
            .search(&query_array, top_k)
            .map_err(|e| PyErr::new::<PyException, _>(format!("Search failed: {}", e)))?;
        Ok(results)
    }
}

#[pyclass]
struct PyFlatAST {
    inner: FlatAST,
}

#[pymethods]
impl PyFlatAST {
    #[new]
    fn new() -> Self {
        Self {
            inner: FlatAST::new(),
        }
    }

    fn is_empty(&self) -> PyResult<bool> {
        Ok(self.inner.is_empty())
    }

    fn clear(&mut self) -> PyResult<()> {
        self.inner.clear();
        Ok(())
    }

    fn push_node(
        &mut self,
        node_id: u64,
        node_type: u8,
        priority: u8,
        is_subnode: bool,
        child_count: u16,
        payload: &str,
    ) -> PyResult<()> {
        self.inner.push_node(
            node_id,
            node_type,
            priority,
            is_subnode,
            child_count,
            payload,
        );
        Ok(())
    }

    fn logical_delete_node(&mut self, target_id: u64) -> PyResult<bool> {
        Ok(self.inner.logical_delete_node(target_id))
    }

    fn total_tokens(&self) -> PyResult<u32> {
        Ok(self.inner.total_tokens())
    }

    fn prune(&mut self, target_token_limit: u32) -> PyResult<u32> {
        Ok(self.inner.prune(target_token_limit))
    }

    fn render_dsl(&self) -> PyResult<String> {
        Ok(self.inner.render_dsl())
    }

    fn render_human_readable(&self) -> PyResult<String> {
        Ok(self.inner.render_human_readable())
    }
}

#[pyfunction]
fn calculate_fnv1a(s: &str) -> u64 {
    tanuki_core::calculate_fnv1a(s)
}

#[pymodule]
fn tanuki_rust(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyTanukiEngine>()?;
    m.add_class::<PyFlatAST>()?;
    m.add_function(wrap_pyfunction!(calculate_fnv1a, m)?)?;
    Ok(())
}
