#pragma once

#include <QJsonObject>
#include <QList>
#include <QString>
#include <QStringList>

// What `chxchx-tech bridge local-models` reports: the machine, Ollama and the
// catalog models assessed for it. Parsing lives apart from the widget so it can
// be tested without a window.
struct LocalModelRow {
    QString tag;
    QString label;
    QString note;
    QString fit; // gpu | partial | cpu | none
    QString summary;
    qint64 sizeBytes = 0;
    qint64 requiredBytes = 0;
    bool diskOk = true;
    bool recommended = false;
    bool downloaded = false;
    bool configured = false;
    bool loaded = false; // currently in Ollama's memory
};

struct LoadedModelInfo {
    QString name;
    qint64 sizeBytes = 0;
    qint64 vramBytes = 0;
};

struct LocalModelsSnapshot {
    QString error; // set when the bridge answered with an error payload
    qint64 totalRamBytes = 0;
    qint64 availableRamBytes = 0;
    qint64 diskFreeBytes = 0;
    int cpuCores = 0;
    QString accelerator;
    QString modelsDir;
    bool ollamaInstalled = false;
    bool ollamaRunning = false;
    QString ollamaVersion;
    QString ollamaUrl;
    QString installCommand; // empty where the official route is a download page
    QString installUrl;
    bool opencodeInstalled = false;
    QString opencodeHint;
    int contextTokens = 32768;
    QList<LocalModelRow> models;
    QList<LoadedModelInfo> loaded;

    bool valid() const { return error.isEmpty() && !models.isEmpty(); }
    bool canSetUp() const { return ollamaInstalled && ollamaRunning; }
};

// One JSONL line of `local-models setup --json`.
struct LocalModelsEvent {
    QString type; // step | progress | notice | preview | done | error
    QString step; // download | context | config
    QString model;
    QString status;
    QString message;
    QString content;
    qint64 completed = 0;
    qint64 total = 0;
    bool changed = false;
    bool dryRun = false;
    bool removal = false; // `done` of `local-models remove`
    QStringList removed;
    QString configPath;
    QString backupPath;
};

LocalModelsSnapshot parseLocalModelsPayload(const QJsonObject &payload);
// Empty `type` means the line was not a local-models event.
LocalModelsEvent parseLocalModelsEvent(const QJsonObject &line);
QString formatBytes(qint64 bytes);
QString fitLabel(const QString &fit);
// Progress of a download as 0-100, or -1 while the size is unknown.
int percentOf(qint64 completed, qint64 total);
