#include "../src/integrations/local_models_model.hpp"
#include "../src/integrations/local_models_widget.hpp"

#include <QApplication>
#include <QElapsedTimer>
#include <QFile>
#include <QLabel>
#include <QJsonArray>
#include <QJsonDocument>
#include <QPushButton>
#include <QTemporaryDir>
#include <QTreeWidget>

#include <cstdio>

namespace {

int fail(int code, const char *what)
{
    std::fprintf(stderr, "local models smoke %d: %s\n", code, what);
    return code;
}

QJsonObject model(const QString &tag, const QString &fit, bool recommended, bool downloaded = false, bool loaded = false)
{
    return {{"tag", tag}, {"label", tag.toUpper()}, {"note", "nota"}, {"fit", fit}, {"summary", "resumen"},
            {"size_bytes", 5.0 * 1024 * 1024 * 1024}, {"required_bytes", 7.0 * 1024 * 1024 * 1024}, {"disk_ok", true},
            {"recommended", recommended}, {"downloaded", downloaded}, {"configured", false}, {"loaded", loaded}};
}

QJsonObject payload(bool installed, bool running)
{
    return {{"schema", "chxchx.local-models"}, {"schema_version", 1}, {"context", 32768},
        {"hardware", QJsonObject{{"total_ram_bytes", 32.0 * 1024 * 1024 * 1024}, {"available_ram_bytes", 20.0 * 1024 * 1024 * 1024},
                                  {"cpu_cores", 8}, {"disk_free_bytes", 400.0 * 1024 * 1024 * 1024},
                                  {"accelerator", "RTX 4070 (12 GB)"}, {"models_dir", "/m"}}},
        {"ollama", QJsonObject{{"installed", installed}, {"running", running}, {"version", "0.12.6"},
                               {"base_url", "http://127.0.0.1:11434"},
                               {"loaded", QJsonArray{QJsonObject{{"name", "qwen3-coder:30b-ctx32k"}, {"size_bytes", 22.0 * 1024 * 1024 * 1024}, {"vram_bytes", 12.0 * 1024 * 1024 * 1024}}}},
                               {"install", QJsonObject{{"command", "curl -fsSL https://ollama.com/install.sh | sh"}, {"url", "https://ollama.com/download"}}}}},
        {"opencode", QJsonObject{{"installed", false}, {"install_hint", "npm install -g opencode-ai"}}},
        {"models", QJsonArray{model("qwen3-coder:30b", "gpu", true, true, true), model("gpt-oss:120b", "none", false)}}};
}

QPushButton *button(QWidget &root, const QString &text)
{
    for (auto *candidate : root.findChildren<QPushButton *>()) {
        if (candidate->text() == text) return candidate;
    }
    return nullptr;
}

QByteArray line(QJsonObject fields)
{
    fields.insert("schema", "chxchx.local-models-event");
    fields.insert("schema_version", 1);
    return QJsonDocument(fields).toJson(QJsonDocument::Compact);
}

} // namespace

int main(int argc, char **argv)
{
    QApplication application(argc, argv);

    if (formatBytes(5LL * 1024 * 1024 * 1024) != QStringLiteral("5.0 GB") || formatBytes(300LL * 1024 * 1024) != QStringLiteral("300 MB")
        || percentOf(50, 200) != 25 || percentOf(5, 0) != -1 || percentOf(500, 100) != 100
        || fitLabel("partial") != QStringLiteral("GPU + RAM") || fitLabel("x") != QStringLiteral("No cabe")) return fail(1, "formatting");

    const LocalModelsSnapshot good = parseLocalModelsPayload(payload(true, true));
    if (!good.valid() || !good.canSetUp() || good.models.size() != 2 || !good.models[0].recommended
        || good.accelerator != QStringLiteral("RTX 4070 (12 GB)") || good.installCommand.isEmpty()) return fail(2, "parse good payload");
    const LocalModelsSnapshot bad = parseLocalModelsPayload({{"schema", "chxchx.error"}, {"error", "boom"}});
    if (bad.valid() || bad.error != QStringLiteral("boom")) return fail(3, "parse error payload");
    if (parseLocalModelsPayload({{"schema", "otro"}}).valid()) return fail(4, "unknown schema");

    const LocalModelsEvent progress = parseLocalModelsEvent(QJsonDocument::fromJson(
        line({{"type", "progress"}, {"status", "pulling"}, {"completed", 5}, {"total", 10}})).object());
    if (progress.type != QStringLiteral("progress") || percentOf(progress.completed, progress.total) != 50) return fail(5, "event parse");
    if (!parseLocalModelsEvent({{"type", "progress"}}).type.isEmpty()) return fail(6, "foreign event must be ignored");
    if (parseLocalModelsEvent(QJsonDocument::fromJson(line({{"type", "preview"}, {"path", "/p/opencode.json"}})).object()).configPath
        != QStringLiteral("/p/opencode.json")) return fail(19, "preview path");

    // Ollama missing: install guidance, setup disabled, preview disabled.
    LocalModelsWidget widget(QStringLiteral("/tmp"), QString());
    widget.resize(900, 700);
    widget.show();
    widget.applyPayload(payload(false, false));
    if (button(widget, "Copiar comando") == nullptr || button(widget, "Copiar comando")->isHidden()) return fail(7, "install command offered");
    if (button(widget, "Descargar y configurar")->isEnabled() || button(widget, "Vista previa")->isEnabled()) return fail(8, "actions without ollama");

    // Installed but stopped: preview works (read-only), real setup does not.
    widget.applyPayload(payload(true, false));
    if (!button(widget, "Vista previa")->isEnabled() || button(widget, "Descargar y configurar")->isEnabled()) return fail(9, "stopped server");

    // Ready: the recommended model is selected and everything is enabled.
    widget.applyPayload(payload(true, true));
    if (widget.selectedRow() == nullptr || widget.selectedRow()->tag != QStringLiteral("qwen3-coder:30b")) return fail(10, "recommended preselected");
    if (!button(widget, "Descargar y configurar")->isEnabled() || widget.findChild<QTreeWidget *>()->topLevelItemCount() != 2) return fail(11, "ready state");
    if (!widget.findChild<QTreeWidget *>()->topLevelItem(0)->text(0).startsWith(QStringLiteral("★"))) return fail(12, "star marks the recommendation");

    // Loaded models show in the header and the table; Quitar needs something to remove.
    if (!widget.findChild<QLabel *>()->text().isEmpty() && widget.snapshot().loaded.size() != 1) return fail(20, "loaded parsed");
    bool headerMentionsLoaded = false;
    for (auto *label : widget.findChildren<QLabel *>()) {
        if (label->text().contains(QStringLiteral("En memoria ahora: qwen3-coder:30b-ctx32k (22.0 GB, 12.0 GB en GPU)"))) headerMentionsLoaded = true;
    }
    if (!headerMentionsLoaded) return fail(21, "loaded models in the header");
    auto *table = widget.findChild<QTreeWidget *>();
    if (table->topLevelItem(0)->text(3) != QStringLiteral("En memoria · Descargado") || !table->topLevelItem(1)->text(3).isEmpty()) return fail(22, "state column");
    if (!button(widget, "Quitar")->isEnabled()) return fail(23, "remove enabled for a downloaded model");
    table->setCurrentItem(table->topLevelItem(1));
    if (button(widget, "Quitar")->isEnabled()) return fail(24, "remove disabled for a model that is not on disk");
    table->setCurrentItem(table->topLevelItem(0));
    widget.applyPayload(payload(true, false));
    if (button(widget, "Quitar")->isEnabled()) return fail(25, "remove needs a running Ollama");
    widget.applyPayload(payload(true, true));
    widget.handleSetupLine(line({{"type", "step"}, {"step", "delete"}, {"model", "qwen3-coder:30b-ctx32k"}}));
    if (widget.progressText() != QStringLiteral("Borrando qwen3-coder:30b-ctx32k de Ollama")) return fail(26, "delete step");
    widget.handleSetupLine(line({{"type", "done"}, {"model", "qwen3-coder:30b"}, {"removed", QJsonArray{"a", "b"}}, {"changed", true},
                                 {"config_path", "/p/opencode.json"}, {"backup_path", QJsonValue::Null}, {"dry_run", false}}));
    if (widget.progressText() != QStringLiteral("Listo. Quitado: a, b")) return fail(27, "removal summary");
    if (!parseLocalModelsEvent(QJsonDocument::fromJson(line({{"type", "done"}, {"removed", QJsonArray{}}})).object()).removal) return fail(28, "removal flag");
    if (parseLocalModelsEvent(QJsonDocument::fromJson(line({{"type", "done"}})).object()).removal) return fail(29, "setup is not a removal");

    // Optional QA capture of the ready state: CHXCHX_LOCAL_MODELS_SCREENSHOT=/path/shot.png
    const QByteArray shot = qgetenv("CHXCHX_LOCAL_MODELS_SCREENSHOT");
    if (!shot.isEmpty()) {
        widget.handleSetupLine(line({{"type", "step"}, {"step", "download"}, {"model", "qwen3-coder:30b"}}));
        widget.handleSetupLine(line({{"type", "progress"}, {"status", "pulling 3f1a"}, {"completed", 7516192768.0}, {"total", 20401094656.0}}));
        QApplication::processEvents();
        widget.grab().save(QString::fromUtf8(shot));
    }

    // Events drive progress and the log.
    widget.handleSetupLine(line({{"type", "step"}, {"step", "download"}, {"model", "qwen3-coder:30b"}}));
    widget.handleSetupLine(line({{"type", "progress"}, {"model", "m"}, {"status", "pulling abc"}, {"completed", 1073741824.0}, {"total", 4294967296.0}}));
    if (!widget.progressText().contains(QStringLiteral("1.0 GB de 4.0 GB"))) return fail(13, "progress text");
    widget.handleSetupLine(line({{"type", "preview"}, {"path", "/p/opencode.json"}, {"content", "{ \"model\": \"x\" }"}}));
    widget.handleSetupLine(line({{"type", "done"}, {"model", "ollama/x"}, {"changed", true}, {"config_path", "/p/opencode.json"}, {"backup_path", "/b"}, {"dry_run", false}}));
    if (!widget.logText().contains(QStringLiteral("opencode.json")) || !widget.logText().contains(QStringLiteral("Backup del archivo anterior: /b"))
        || !widget.progressText().startsWith(QStringLiteral("Listo"))) return fail(14, "done summary");
    widget.handleSetupLine(line({{"type", "error"}, {"message", "Ollama no responde"}}));
    if (widget.progressText() != QStringLiteral("Ollama no responde")) return fail(15, "error shown");

    // A real refresh through a fake CLI.
#ifndef Q_OS_WIN
    QTemporaryDir directory;
    const QString script = directory.filePath("fake-cli.sh");
    QFile file(script);
    if (!file.open(QIODevice::WriteOnly)) return fail(16, "script");
    file.write("#!/bin/sh\ncat <<'JSON'\n" + QJsonDocument(payload(true, true)).toJson(QJsonDocument::Compact) + "\nJSON\n");
    file.setPermissions(file.permissions() | QFileDevice::ExeUser);
    file.close();
    LocalModelsWidget live(QStringLiteral("/tmp"), script);
    QElapsedTimer timer;
    timer.start();
    while (live.snapshot().models.isEmpty() && timer.elapsed() < 10000) QApplication::processEvents(QEventLoop::AllEvents, 50);
    if (live.snapshot().models.size() != 2) return fail(17, "refresh through the CLI");
#endif

    // Without a CLI the tab explains instead of hanging.
    LocalModelsWidget missing(QStringLiteral("/tmp"), QString());
    if (missing.snapshot().error.isEmpty()) return fail(18, "missing CLI is reported");
    return 0;
}
