#pragma once

#include "local_models_model.hpp"

#include <QProcess>
#include <QWidget>

class QCheckBox;
class QFrame;
class QLabel;
class QPlainTextEdit;
class QProgressBar;
class QPushButton;
class QTreeWidget;

// "Modelos locales" tab: shows what this machine can run, whether Ollama is
// ready, and sets a model up for OpenCode. Detection, downloads and the
// `opencode.json` merge all stay in the Python CLI; this widget only runs
// `bridge local-models` and `local-models setup --json` and shows the result.
class LocalModelsWidget final : public QWidget {
    Q_OBJECT
public:
    LocalModelsWidget(QString projectPath, QString cliProgram, QWidget *parent = nullptr);
    ~LocalModelsWidget() override;

    void refresh();
    // Entry point for tests and for `refresh`: renders an already-fetched payload.
    void applyPayload(const QJsonObject &payload);
    bool isBusy() const;
    const LocalModelsSnapshot &snapshot() const { return m_snapshot; }
    // Selected catalog row, or nullptr.
    const LocalModelRow *selectedRow() const;
    QString progressText() const;
    QString logText() const;
    // Feeds one `local-models setup --json` line, as the process would.
    void handleSetupLine(const QByteArray &line);

private:
    void buildUi();
    void updateBanner();
    void updateDetail();
    void updateActions();
    void startSetup(bool dryRun);
    void startRemove();
    void runCommand(const QStringList &arguments, const QString &firstProgress);
    bool confirmSetup(const LocalModelRow &row, bool &force);
    void finishSetup(int exitCode);
    void appendLog(const QString &line);
    void setProgress(const QString &text, int percent);

    QString m_projectPath;
    QString m_cli;
    LocalModelsSnapshot m_snapshot;
    QProcess m_refreshProcess;
    QProcess m_setupProcess;
    QByteArray m_setupBuffer;
    bool m_setupDone = false;
    bool m_setupFailed = false;

    QLabel *m_hardware = nullptr;
    QFrame *m_banner = nullptr;
    QLabel *m_bannerText = nullptr;
    QPushButton *m_copyInstall = nullptr;
    QPushButton *m_openDownload = nullptr;
    QTreeWidget *m_table = nullptr;
    QLabel *m_detail = nullptr;
    QCheckBox *m_setDefault = nullptr;
    QPushButton *m_preview = nullptr;
    QPushButton *m_apply = nullptr;
    QPushButton *m_remove = nullptr;
    QPushButton *m_cancel = nullptr;
    QPushButton *m_reload = nullptr;
    QProgressBar *m_progress = nullptr;
    QLabel *m_progressLabel = nullptr;
    QPlainTextEdit *m_log = nullptr;
};
