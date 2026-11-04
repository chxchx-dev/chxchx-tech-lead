#pragma once

#include <QList>
#include <QString>
#include <QStringList>

// The assistant asks short questions (a commit message, a piece of advice), so by default it should
// not spend the agent's biggest model on them. These helpers choose a light model from the list the
// agent itself publishes (`bridge agent-options`), and the lowest reasoning effort it accepts.

struct GitAssistantModel {
    QString id;
    QStringList efforts;
};

struct GitModelList {
    QList<GitAssistantModel> models;
    QString defaultModel;
    bool loaded = false;
};

// What to pass to the agent. An empty `model` means "the agent's own default".
struct GitModelChoice {
    QString model;
    QString effort;
    bool light = false;  // a model picked because it looks light, not the agent's default
};

// Marker stored when the person wants the agent's own default model.
inline constexpr char GitAgentDefaultModel[] = "__agent__";

GitModelList parseAgentOptions(const QByteArray &json);
// Names that usually mean a small, fast or cheap model (mini, nano, haiku, flash, lite…) or a small local one (7b).
bool looksLight(const QString &modelId);
// "low" (or "minimal") when the list has it; empty when the model takes no effort setting.
QString lowEffort(const QStringList &efforts);
// `preference`: empty = automatic (a light model), GitAgentDefaultModel, or a model id.
GitModelChoice chooseModel(const GitModelList &list, const QString &preference);
// "gpt-mini · esfuerzo bajo", for tooltips and notes.
QString describeChoice(const GitModelChoice &choice);
