import httpx
from typing import List, Dict, Any, Optional

class TanukiClient:
    def __init__(self, base_url: str = "http://localhost:3000"):
        """T.A.N.U.K.I. API Client SDK
        
        Args:
            base_url (str): Target URL of the tanuki-serving API server.
        """
        self.base_url = base_url.rstrip("/")
        # Initialize httpx AsyncClient
        self.client = httpx.AsyncClient(base_url=self.base_url, timeout=30.0)

    async def close(self):
        """Close the HTTP client session."""
        await self.client.aclose()

    async def __aenter__(self):
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.close()

    async def health(self) -> str:
        """Check serving API health status."""
        response = await self.client.get("/health")
        response.raise_for_status()
        return response.text

    async def get_nodes(self) -> List[Dict[str, Any]]:
        """Fetch all knowledge nodes from the database."""
        response = await self.client.get("/api/nodes")
        response.raise_for_status()
        return response.json()

    async def get_clusters(self) -> List[Dict[str, Any]]:
        """Fetch all knowledge clusters from the database."""
        response = await self.client.get("/api/clusters")
        response.raise_for_status()
        return response.json()

    async def search(self, query: str) -> List[Dict[str, Any]]:
        """Perform text keyword AND search on nodes."""
        response = await self.client.get("/api/search", params={"q": query})
        response.raise_for_status()
        return response.json()

    async def vector_search(self, vector: List[float], top_k: int = 5) -> List[Dict[str, Any]]:
        """Perform structured mmap vector search on nodes.
        
        Args:
            vector (List[float]): 768-dimensional embedding query vector.
            top_k (int): Maximum number of results to return.
        """
        payload = {"vector": vector, "top_k": top_k}
        response = await self.client.post("/api/vector-search", json=payload)
        response.raise_for_status()
        return response.json()
