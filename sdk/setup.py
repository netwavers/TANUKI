from setuptools import setup, find_packages

setup(
    name="tanuki",
    version="0.1.0",
    description="T.A.N.U.K.I. API Client SDK",
    author="たぬきちゃん",
    packages=find_packages(),
    install_requires=[
        "httpx>=0.20.0",
    ],
    python_requires=">=3.7",
)
