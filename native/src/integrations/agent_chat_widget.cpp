#include "agent_chat_widget.hpp"

#include <QComboBox>
#include <QFileInfo>
#include <QHBoxLayout>
#include <QJsonDocument>
#include <QKeySequence>
#include <QLabel>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QScrollBar>
#include <QShortcut>
#include <QTextBrowser>
#include <QTimer>
#include <QVBoxLayout>

namespace {

constexpr int RenderDelayMs = 40;
constexpr int StopTimeoutMs = 5000;

} // namespace

AgentChatWidget::AgentChatWidget(QString projectPath, QString agentId, QString cliProgram, QWidget *parent)
    : QWidget(parent)
    , m_projectPath(std::move(projectPath))
    , m_agentId(std::move(agentId))
    , m_cliProgram(std::move(cliProgram))
{
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(8, 8, 8, 8);
    layout->setSpacing(6);

    auto *header = new QHBoxLayout;
    auto *title = new QLabel(QStringLiteral("<b>%1</b> · chat visual").arg(m_agentId.toHtmlEscaped()), this);
    m_mode = new QComboBox(this);
    m_mode->addItem(QStringLiteral("Solo lectura"), QStringLiteral("read-only"));
    m_mode->addItem(QStringLiteral("Editar workspace"), QStringLiteral("workspace-write"));
    m_mode->setToolTip(QStringLiteral(
        "Solo lectura: el agente puede leer y proponer, sin modificar archivos.\n"
        "Editar workspace: puede modificar archivos del proyecto (sandbox de Codex / acceptEdits de Claude)."));
    m_newChat = new QPushButton(QStringLiteral("Nuevo chat"), this);
    m_newChat->setToolTip(QStringLiteral("Empieza otra conversación con el contexto del proyecto."));
    header->addWidget(title);
    header->addStretch(1);
    header->addWidget(new QLabel(QStringLiteral("Permisos:"), this));
    header->addWidget(m_mode);
    header->addWidget(m_newChat);
    layout->addLayout(header);

    m_transcript = new QTextBrowser(this);
    m_transcript->setOpenExternalLinks(true);
    m_transcript->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    m_transcript->setStyleSheet(QStringLiteral("QTextBrowser { background: #0d1117; border: 1px solid #262c36; }"));
    m_transcript->setPlaceholderText(QStringLiteral(
        "Escribe tu primera solicitud. El agente recibirá el contexto del proyecto (estado, handoff, memoria y últimas sesiones)."));
    layout->addWidget(m_transcript, 1);

    m_status = new QLabel(QStringLiteral("Listo."), this);
    m_status->setStyleSheet(QStringLiteral("color: #8d96a8;"));
    layout->addWidget(m_status);

    auto *inputRow = new QHBoxLayout;
    m_input = new QPlainTextEdit(this);
    m_input->setPlaceholderText(QStringLiteral("Mensaje para el agente… (Ctrl+Enter para enviar)"));
    m_input->setMaximumHeight(110);
    m_send = new QPushButton(QStringLiteral("Enviar"), this);
    m_send->setMinimumHeight(36);
    inputRow->addWidget(m_input, 1);
    inputRow->addWidget(m_send);
    layout->addLayout(inputRow);

    m_renderTimer = new QTimer(this);
    m_renderTimer->setSingleShot(true);
    m_renderTimer->setInterval(RenderDelayMs);
    connect(m_renderTimer, &QTimer::timeout, this, &AgentChatWidget::render);

    auto *sendShortcut = new QShortcut(QKeySequence(Qt::CTRL | Qt::Key_Return), m_input);
    sendShortcut->setContext(Qt::WidgetShortcut);
    connect(sendShortcut, &QShortcut::activated, m_send, &QPushButton::click);
    connect(m_send, &QPushButton::clicked, this, [this] {
        if (isRunning()) {
            stop();
            return;
        }
        sendMessage(m_input->toPlainText());
    });
    connect(m_newChat, &QPushButton::clicked, this, &AgentChatWidget::startNewChat);

    m_process.setWorkingDirectory(m_projectPath);
    m_process.setProcessChannelMode(QProcess::SeparateChannels);
    connect(&m_process, &QProcess::readyReadStandardOutput, this, &AgentChatWidget::readOutput);
    connect(&m_process, qOverload<int, QProcess::ExitStatus>(&QProcess::finished),
        this, &AgentChatWidget::finishTurn);
    connect(&m_process, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (error != QProcess::FailedToStart) return;
        QJsonObject event{{QStringLiteral("schema"), QStringLiteral("chxchx.agent-event")},
            {QStringLiteral("schema_version"), 1}, {QStringLiteral("type"), QStringLiteral("error")},
            {QStringLiteral("message"), QStringLiteral("No se pudo iniciar chxchx-tech: %1").arg(m_process.errorString())}};
        m_model.apply(event);
        setRunning(false);
        scheduleRender();
    });
}

bool AgentChatWidget::supportsProgram(const QString &program)
{
    QString name = QFileInfo(QString(program).replace(QLatin1Char('\\'), QLatin1Char('/'))).fileName().toLower();
    for (const auto &suffix : {QStringLiteral(".exe"), QStringLiteral(".cmd"), QStringLiteral(".bat")}) {
        if (name.endsWith(suffix)) name.chop(suffix.size());
    }
    return name == QStringLiteral("codex") || name == QStringLiteral("claude");
}

AgentChatWidget::~AgentChatWidget()
{
    if (!isRunning()) return;
    // Closing the tab must not leave the agent working unattended.
    m_process.write("cancel\n");
    m_process.closeWriteChannel();
    if (!m_process.waitForFinished(StopTimeoutMs)) {
        m_process.kill();
        m_process.waitForFinished(1000);
    }
}

bool AgentChatWidget::isRunning() const
{
    return m_process.state() != QProcess::NotRunning;
}

void AgentChatWidget::focusInput()
{
    m_input->setFocus(Qt::OtherFocusReason);
}

void AgentChatWidget::sendMessage(const QString &text)
{
    const QString message = text.trimmed();
    if (message.isEmpty() || isRunning()) return;
    if (m_cliProgram.isEmpty()) {
        m_status->setText(QStringLiteral("No se encontró chxchx-tech. Configura CHXCHX_TECH_CLI o añádelo al PATH."));
        return;
    }
    m_model.addUserMessage(message);
    m_input->clear();
    QStringList arguments = {QStringLiteral("agent"), QStringLiteral("chat"), m_agentId,
        QStringLiteral("--path"), m_projectPath, QStringLiteral("--prompt"), message,
        QStringLiteral("--mode"), m_mode->currentData().toString(), QStringLiteral("--cancel-on-stdin")};
    if (!m_model.sessionId().isEmpty()) arguments << QStringLiteral("--session") << m_model.sessionId();
    m_pending.clear();
    m_process.start(m_cliProgram, arguments);
    setRunning(true);
    m_status->setText(m_model.sessionId().isEmpty()
        ? QStringLiteral("Preparando contexto y abriendo sesión…")
        : QStringLiteral("El agente está trabajando…"));
    render();
}

void AgentChatWidget::stop()
{
    if (!isRunning()) return;
    m_process.write("cancel\n");
    m_status->setText(QStringLiteral("Deteniendo al agente…"));
}

void AgentChatWidget::startNewChat()
{
    if (isRunning()) return;
    m_model.clear();
    m_transcript->clear();
    m_status->setText(QStringLiteral("Chat nuevo: el próximo mensaje recibirá el contexto del proyecto."));
    focusInput();
}

void AgentChatWidget::readOutput()
{
    m_pending += m_process.readAllStandardOutput();
    qsizetype newline = -1;
    bool changed = false;
    while ((newline = m_pending.indexOf('\n')) >= 0) {
        const QByteArray line = m_pending.left(newline).trimmed();
        m_pending.remove(0, newline + 1);
        const auto document = QJsonDocument::fromJson(line);
        if (document.isObject()) changed = m_model.apply(document.object()) || changed;
    }
    if (changed) scheduleRender();
}

void AgentChatWidget::finishTurn(int exitCode, QProcess::ExitStatus status)
{
    readOutput();
    const QString stderrText = QString::fromUtf8(m_process.readAllStandardError()).trimmed();
    const bool reported = !m_model.entries().isEmpty()
        && m_model.entries().constLast().kind == AgentChatModel::Kind::Error;
    if ((status != QProcess::NormalExit || exitCode != 0) && !reported && !stderrText.isEmpty()) {
        m_model.apply(QJsonObject{{QStringLiteral("schema"), QStringLiteral("chxchx.agent-event")},
            {QStringLiteral("schema_version"), 1}, {QStringLiteral("type"), QStringLiteral("error")},
            {QStringLiteral("message"), stderrText.right(2000)}});
    }
    setRunning(false);
    const QString usage = m_model.usageSummary();
    m_status->setText(usage.isEmpty() ? QStringLiteral("Listo.") : QStringLiteral("Listo · %1").arg(usage));
    render();
    focusInput();
}

void AgentChatWidget::scheduleRender()
{
    if (!m_renderTimer->isActive()) m_renderTimer->start();
}

void AgentChatWidget::render()
{
    QScrollBar *scroll = m_transcript->verticalScrollBar();
    const bool atBottom = scroll->value() >= scroll->maximum() - 4;
    const int previous = scroll->value();
    m_transcript->setHtml(m_model.renderHtml());
    scroll->setValue(atBottom ? scroll->maximum() : previous);
}

void AgentChatWidget::setRunning(bool running)
{
    m_send->setText(running ? QStringLiteral("Detener") : QStringLiteral("Enviar"));
    m_newChat->setEnabled(!running);
    emit runningStateChanged(running);
}
