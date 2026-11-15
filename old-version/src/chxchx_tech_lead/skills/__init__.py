"""Skill discovery and project-specific recommendations."""

from .registry import Skill, SkillRecommendation, SkillRegistry
from .project import ProjectSkillResult, enable_skills, enabled_skills, set_skill_enabled, sync_project_skills
from .packs import PackRecommendation, TechPack, TechPackRegistry

__all__ = [
    "PackRecommendation",
    "ProjectSkillResult",
    "Skill",
    "SkillRecommendation",
    "SkillRegistry",
    "TechPack",
    "TechPackRegistry",
    "enable_skills",
    "enabled_skills",
    "set_skill_enabled",
    "sync_project_skills",
]
