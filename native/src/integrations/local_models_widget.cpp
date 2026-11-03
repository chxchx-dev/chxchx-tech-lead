#include "local_models_widget.hpp"

#include "local_models_widget_style.hpp"

#include <QApplication>
#include <QCheckBox>
#include <QClipboard>
#include <QDesktopServices>
#include <QFrame>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonDocument>
#include <QLabel>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QProgressBar>
#include <QPushButton>
#include <QTimer>
#include <QTreeWidget>
#include <QUrl>
#include <QVBoxLayout>

LocalModelsWidget::LocalModelsWidget(QString projectPath, QString cliProgram, QWidget *parent)
    : QWidget(parent), m_projectPath(std::move(projectPath)), m_cli(std::move(cliProgram))
{
    setObjectName(QStringLiteral("localModelsWidget"));
    buildUi();
    m_refreshProcess.setProcessChannelMode(QProcess::SeparateChannels);
    connect(&m_refreshProcess, qOverload<int, QProcess::ExitStatus>(&QProcess::finished), this, [this] {
        const QByteArray output = m_refreshProcess.readAllStandardOutput();
        QJsonObject payload = QJsonDocument::fromJson(output).object();
        if (payload.isEmpty()) {
            payload = {{QStringLiteral("schema"), QStringLiteral("chxchx.error")},
                       {QStringLiteral("error"), QString::fromUtf8(m_refreshProcess.readAllStandardError()).trimmed()}};
        }
        applyPayload(payload);
    });
    m_setupProcess.setProcessChannelMode(QProcess::SeparateChannels);
    connect(&m_setupProcess, &QProcess::readyReadStandardOutput, this, [this] {
        m_setupBuffer += m_setupProcess.readAllStandardOutput();
        int newline;
        while ((newline = m_setupBuffer.indexOf('\n')) >= 0) {
            handleSetupLine(m_setupBuffer.left(newline));
            m_setupBuffer.remove(0, newline + 1);
        }
    });
    connect(&m_setupProcess, qOverload<int, QProcess::ExitStatus>(&QProcess::finished), this,
        [this](int exitCode) { finishSetup(exitCode); });
    connect(&m_setupProcess, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart) finishSetup(127);
    });
    refresh();
}

LocalModelsWidget::~LocalModelsWidget()
{
    for (QProcess *process : {&m_refreshProcess, &m_setupProcess}) {
        if (process->state() != QProcess::NotRunning) {
            process->kill();
            process->waitForFinished(1000);
        }
    }
}

void LocalModelsWidget::buildUi()
{
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(18, 14, 18, 14);
    layout->setSpacing(10);
    auto *title = new QLabel(QStringLiteral("Modelos locales para OpenCode"), this);
    title->setObjectName(QStringLiteral("localModelsTitle"));
    QFont font = title->font();
    font.setPointSizeF(font.pointSizeF() * 1.4);
    font.setBold(true);
    title->setFont(font);
    layout->addWidget(title);
    m_hardware = new QLabel(QStringLiteral("Detectando hardware…"), this);
    m_hardware->setWordWrap(true);
    layout->addWidget(m_hardware);

    m_banner = new QFrame(this);
    m_banner->setObjectName(QStringLiteral("localModelsBanner"));
    auto *bannerLayout = new QHBoxLayout(m_banner);
    bannerLayout->setContentsMargins(12, 8, 12, 8);
    m_bannerText = new QLabel(m_banner);
    m_bannerText->setWordWrap(true);
    m_bannerText->setTextInteractionFlags(Qt::TextSelectableByMouse);
    bannerLayout->addWidget(m_bannerText, 1);
    m_copyInstall = new QPushButton(QStringLiteral("Copiar comando"), m_banner);
    m_openDownload = new QPushButton(QStringLiteral("Abrir descarga"), m_banner);
    bannerLayout->addWidget(m_copyInstall);
    bannerLayout->addWidget(m_openDownload);
    layout->addWidget(m_banner);
    const auto styleBanner = [this] {
        const QColor warning = ThemeManager::current().warning;
        m_banner->setStyleSheet(QStringLiteral("#localModelsBanner { border: 1px solid %1; border-radius: 6px; }")
                                    .arg(warning.name()));
    };
    styleBanner();
    connect(ThemeManager::instance(), &ThemeManager::themeChanged, this, styleBanner);
    connect(m_copyInstall, &QPushButton::clicked, this, [this] {
        QApplication::clipboard()->setText(m_snapshot.installCommand);
    });
    connect(m_openDownload, &QPushButton::clicked, this, [this] {
        QDesktopServices::openUrl(QUrl(m_snapshot.installUrl));
    });

    m_table = new QTreeWidget(this);
    m_table->setRootIsDecorated(false);
    m_table->setAlternatingRowColors(true);
    m_table->setHeaderLabels({QStringLiteral("Modelo"), QStringLiteral("Descarga"), QStringLiteral("Encaje"), QStringLiteral("Estado")});
    m_table->header()->setSectionResizeMode(ColumnModel, QHeaderView::Stretch);
    layout->addWidget(m_table, 1);
    connect(m_table, &QTreeWidget::currentItemChanged, this, [this] { updateDetail(); updateActions(); });

    m_detail = new QLabel(this);
    m_detail->setWordWrap(true);
    layout->addWidget(m_detail);
    m_setDefault = new QCheckBox(QStringLiteral("Usar como modelo predeterminado de OpenCode en este proyecto"), this);
    layout->addWidget(m_setDefault);

    auto *buttons = new QHBoxLayout;
    m_preview = new QPushButton(QStringLiteral("Vista previa"), this);
    m_preview->setToolTip(QStringLiteral("Muestra los pasos y el opencode.json resultante sin cambiar nada (--dry-run)."));
    m_apply = new QPushButton(QStringLiteral("Descargar y configurar"), this);
    m_apply->setDefault(true);
    m_apply->setMinimumWidth(m_apply->fontMetrics().horizontalAdvance(m_apply->text()) + 56); // bold default-button text
    m_remove = new QPushButton(QStringLiteral("Quitar"), this);
    m_remove->setToolTip(QStringLiteral("Borra el modelo de Ollama y lo quita de opencode.json del proyecto."));
    m_cancel = new QPushButton(QStringLiteral("Cancelar"), this);
    m_reload = new QPushButton(QStringLiteral("Actualizar"), this);
    buttons->addWidget(m_preview);
    buttons->addWidget(m_apply);
    buttons->addWidget(m_remove);
    buttons->addWidget(m_cancel);
    buttons->addStretch(1);
    buttons->addWidget(m_reload);
    layout->addLayout(buttons);
    connect(m_preview, &QPushButton::clicked, this, [this] { startSetup(true); });
    connect(m_apply, &QPushButton::clicked, this, [this] { startSetup(false); });
    connect(m_remove, &QPushButton::clicked, this, [this] { startRemove(); });
    connect(m_reload, &QPushButton::clicked, this, [this] { refresh(); });
    connect(m_cancel, &QPushButton::clicked, this, [this] {
        appendLog(QStringLiteral("Cancelado. Lo ya descargado se conserva en Ollama y la descarga se reanuda."));
        m_setupProcess.terminate();
        QTimer::singleShot(3000, &m_setupProcess, &QProcess::kill);
    });

    m_progressLabel = new QLabel(this);
    m_progress = new QProgressBar(this);
    m_progress->setTextVisible(true);
    layout->addWidget(m_progressLabel);
    layout->addWidget(m_progress);
    m_log = new QPlainTextEdit(this);
    m_log->setReadOnly(true);
    m_log->setMaximumBlockCount(500);
    m_log->setMaximumHeight(150);
    m_log->setPlaceholderText(QStringLiteral("Aquí aparecen los pasos y la vista previa de opencode.json."));
    layout->addWidget(m_log);
    setProgress({}, -1);
    m_progress->hide();
    updateActions();
}

void LocalModelsWidget::refresh()
{
    if (m_cli.isEmpty()) {
        applyPayload({{QStringLiteral("schema"), QStringLiteral("chxchx.error")},
                      {QStringLiteral("error"), QStringLiteral("No se encontró chxchx-tech. Configura CHXCHX_TECH_CLI o abre Studio desde el entorno del proyecto.")}});
        return;
    }
    if (m_refreshProcess.state() != QProcess::NotRunning) return;
    m_hardware->setText(QStringLiteral("Detectando hardware…"));
    m_refreshProcess.start(m_cli, {QStringLiteral("bridge"), QStringLiteral("local-models"), m_projectPath});
}

bool LocalModelsWidget::isBusy() const
{
    return m_setupProcess.state() != QProcess::NotRunning;
}

const LocalModelRow *LocalModelsWidget::selectedRow() const
{
    const QTreeWidgetItem *item = m_table->currentItem();
    if (item == nullptr) return nullptr;
    const QString tag = item->data(ColumnModel, Qt::UserRole).toString();
    for (const LocalModelRow &row : m_snapshot.models) {
        if (row.tag == tag) return &row;
    }
    return nullptr;
}

QString LocalModelsWidget::progressText() const { return m_progressLabel->text(); }
QString LocalModelsWidget::logText() const { return m_log->toPlainText(); }
