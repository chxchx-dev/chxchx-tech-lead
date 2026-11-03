#pragma once

#include "git_assistant_prompts.hpp"
#include "git_types.hpp"

// The next step for the person, decided from the repository state alone (no agent needed).
struct GitGuidance {
    enum class Tone { Done, Info, Warning };
    Tone tone = Tone::Info;
    QString icon;             // studioIcon name
    QString title;
    QStringList steps;
    GitAdviceKind adviceKind = GitAdviceKind::WhatNext;
    bool offersMessageSuggestion = false;  // the assistant can write the commit message
};

struct GitGuideFacts {
    bool mergeInProgress = false;
    bool hasRemotes = false;
    int stashCount = 0;
};

GitGuidance guidanceFor(const GitStatus &status, const GitGuideFacts &facts);
