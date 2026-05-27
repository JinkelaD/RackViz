from pydantic_settings import BaseSettings

class Settings(BaseSettings):
    database_url: str = "sqlite:///./rackviz.db"
    cors_origins: list[str] = ["http://localhost:5173", "http://localhost:3000"]
    icmp_timeout: int = 2
    icmp_count: int = 1

    class Config:
        env_file = ".env"

settings = Settings()