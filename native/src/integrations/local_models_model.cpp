#include "local_models_model.hpp"

#include "bridge_schemas.hpp"

#include <QJsonArray>

namespace {

qint64 integer(const QJsonObject &object, const char *key)
{
    return static_cast<qint64>(object.value(QLatin1StringView(key)).toDouble());
}

QString text(const QJsonObject &object, const char *key)
{
    return object.value(QLatin1StringView(key)).toString();
}

} // namespace

LocalModelsSnapshot parseLocalModelsPayload(const QJsonObject &payload)
{
    LocalModelsSnapshot snapshot;
    const QString schema = text(payload, "schema");
    if (schema == BridgeSchemas::Error) {
        snapshot.error = text(payload, "error");
        if (snapshot.error.isEmpty()) snapshot.error = QStringLiteral("Error desconocido del bridge");
        return snapshot;
    }
    if (schema != BridgeSchemas::LocalModels) {
        snapshot.error = QStringLiteral("Respuesta inesperada del bridge");
        return snapshot;
    }
    const QJsonObject hardware = payload.value(QStringLiteral("hardware")).toObject();
    snapshot.totalRamBytes = integer(hardware, "total_ram_bytes");
    snapshot.availableRamBytes = integer(hardware, "available_ram_bytes");
    snapshot.diskFreeBytes = integer(hardware, "disk_free_bytes");
    snapshot.cpuCores = static_cast<int>(integer(hardware, "cpu_cores"));
    snapshot.accelerator = text(hardware, "accelerator");
    snapshot.modelsDir = text(hardware, "models_dir");

    const QJsonObject ollama = payload.value(QStringLiteral("ollama")).toObject();
    snapshot.ollamaInstalled = ollama.value(QStringLiteral("installed")).toBool();
    snapshot.ollamaRunning = ollama.value(QStringLiteral("running")).toBool();
    snapshot.ollamaVersion = text(ollama, "version");
    snapshot.ollamaUrl = text(ollama, "base_url");
    const QJsonObject install = ollama.value(QStringLiteral("install")).toObject();
    snapshot.installCommand = text(install, "command");
    snapshot.installUrl = text(install, "url");
    for (const auto &value : ollama.value(QStringLiteral("loaded")).toArray()) {
        const QJsonObject item = value.toObject();
        snapshot.loaded.append({text(item, "name"), integer(item, "size_bytes"), integer(item, "vram_bytes")});
    }

    const QJsonObject opencode = payload.value(QStringLiteral("opencode")).toObject();
    snapshot.opencodeInstalled = opencode.value(QStringLiteral("installed")).toBool();
    snapshot.opencodeHint = text(opencode, "install_hint");
    snapshot.contextTokens = payload.value(QStringLiteral("context")).toInt(snapshot.contextTokens);

    for (const auto &value : payload.value(QStringLiteral("models")).toArray()) {
        const QJsonObject item = value.toObject();
        LocalModelRow row;
        row.tag = text(item, "tag");
        row.label = text(item, "label");
        row.note = text(item, "note");
        row.fit = text(item, "fit");
        row.summary = text(item, "summary");
        row.sizeBytes = integer(item, "size_bytes");
        row.requiredBytes = integer(item, "required_bytes");
        row.diskOk = item.value(QStringLiteral("disk_ok")).toBool(true);
        row.recommended = item.value(QStringLiteral("recommended")).toBool();
        row.downloaded = item.value(QStringLiteral("downloaded")).toBool();
        row.configured = item.value(QStringLiteral("configured")).toBool();
        row.loaded = item.value(QStringLiteral("loaded")).toBool();
        if (!row.tag.isEmpty()) snapshot.models.append(row);
    }
    return snapshot;
}

LocalModelsEvent parseLocalModelsEvent(const QJsonObject &line)
{
    LocalModelsEvent event;
    if (text(line, "schema") != BridgeSchemas::LocalModelsEvent) return event;
    event.type = text(line, "type");
    event.step = text(line, "step");
    event.model = text(line, "model");
    event.status = text(line, "status");
    event.message = text(line, "message");
    event.content = text(line, "content");
    event.completed = integer(line, "completed");
    event.total = integer(line, "total");
    event.changed = line.value(QStringLiteral("changed")).toBool();
    event.dryRun = line.value(QStringLiteral("dry_run")).toBool();
    // `done` reports `config_path`; `preview` calls the same file `path`.
    event.configPath = text(line, "config_path");
    if (event.configPath.isEmpty()) event.configPath = text(line, "path");
    event.backupPath = text(line, "backup_path");
    event.removal = line.contains(QStringLiteral("removed"));
    for (const auto &name : line.value(QStringLiteral("removed")).toArray()) event.removed << name.toString();
    return event;
}

QString formatBytes(qint64 bytes)
{
    constexpr double gib = 1024.0 * 1024.0 * 1024.0;
    if (bytes >= static_cast<qint64>(gib)) return QStringLiteral("%1 GB").arg(bytes / gib, 0, 'f', 1);
    return QStringLiteral("%1 MB").arg(bytes / (1024.0 * 1024.0), 0, 'f', 0);
}

QString fitLabel(const QString &fit)
{
    if (fit == QStringLiteral("gpu")) return QStringLiteral("GPU");
    if (fit == QStringLiteral("partial")) return QStringLiteral("GPU + RAM");
    if (fit == QStringLiteral("cpu")) return QStringLiteral("Solo CPU");
    return QStringLiteral("No cabe");
}

int percentOf(qint64 completed, qint64 total)
{
    if (total <= 0) return -1;
    return static_cast<int>(qBound<qint64>(0, completed * 100 / total, qint64(100)));
}
