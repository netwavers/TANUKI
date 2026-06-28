use pyo3::prelude::*;
use tanuki_core::MmapMemoryManager;
use pyo3::exceptions::PyException;

#[pyclass]
struct PyTanukiEngine {
    manager: MmapMemoryManager,
}

#[pymethods]
impl PyTanukiEngine {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let manager = MmapMemoryManager::new(path)
            .map_err(|e| PyErr::new::<PyException, _>(format!("Failed to load memory file: {}", e)))?;
        Ok(Self { manager })
    }

    fn update_mapping(&self, path: &str) -> PyResult<()> {
        self.manager.update_mapping(path)
            .map_err(|e| PyErr::new::<PyException, _>(format!("Failed to update mapping: {}", e)))?;
        Ok(())
    }

    fn search(&self, query_vector: Vec<f32>, top_k: usize) -> PyResult<Vec<(u64, f32)>> {
        if query_vector.len() != 768 {
            return Err(PyErr::new::<PyException, _>("Query vector must be exactly 768 dimensions"));
        }
        let mut query_array = [0.0f32; 768];
        query_array.copy_from_slice(&query_vector[0..768]);

        let results = self.manager.search(&query_array, top_k)
            .map_err(|e| PyErr::new::<PyException, _>(format!("Search failed: {}", e)))?;
        Ok(results)
    }
}

#[pyfunction]
fn calculate_fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[pymodule]
fn tanuki_rust(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyTanukiEngine>()?;
    m.add_function(wrap_pyfunction!(calculate_fnv1a, m)?)?;
    Ok(())
}
