#include "agent_chat_widget.hpp"
#include "agent_chat_composer.hpp"
#include "agent_chat_settings.hpp"
#include "agent_chat_style.hpp"
#include "agent_chat_usage.hpp"
#include "agent_chat_view.hpp"
#include "chat_pill_picker.hpp"

#include <QDir>
#include <QFileInfo>
#include <QHBoxLayout>
#include <QJsonDocument>
#include <QLabel>
#include <QPushButton>
#include <QVBoxLayout>

namespace {

constexpr int StopTimeoutMs = 5000;

QJsonObject localEvent(const QString &type, const QString &message)
{
    return {{QStringLiteral("schema"), QStringLiteral("chxchx.agent-event")}, {QStringLiteral("schema_version"), 1},
        {QStringLiteral("type"), type}, {QStringLiteral("message"), message}};
}

QString providerGuess(const QString &agentId)
{
    const QString id = agentId.toLower();
    if (id.contains(QStringLiteral("claude"))) return QStringLiteral("Claude");
    if (id.contains(QStringLiteral("codex"))) return QStringLiteral("Codex");
    return {};
}

} // namespace

AgentChatWidget::AgentChatWidget(QString projectPath, QString agentId, QString cliProgram, QWidget *parent)
    : QWidget(parent)
    , m_projectPath(std::move(projectPath))
    , m_agentId(std::move(agentId))
    , m_cliProgram(std::move(cliProgram))
    , m_identity(chatIdentityFor(providerGuess(m_agentId), m_agentId))
{
    setObjectName(QStringLiteral("agentChat"));
    setAttribute(Qt::WA_StyledBackground);
    setStyleSheet(agentChatStyleSheet(ThemeManager::current()));
    connect(ThemeManager::instance(), &ThemeManager::themeChanged, this,
        [this](const StudioTheme &theme) { setStyleSheet(agentChatStyleSheet(theme)); });
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);

    auto *header = new QFrame(this);
    header->setObjectName(QStringLiteral("chatHeader"));
    auto *headerColumn = new QVBoxLayout(header);
    headerColumn->setContentsMargins(16, 10, 16, 10);
    headerColumn->setSpacing(8);
    m_headerRow = new QHBoxLayout;
    m_headerRow->setSpacing(10);
    m_avatar = createChatAvatar(m_identity, 32, header);
    auto *titles = new QVBoxLayout;
    titles->setSpacing(0);
    m_title = new QLabel(m_identity.name, header);
    m_title->setObjectName(QStringLiteral("chatTitle"));
    m_subtitle = new QLabel(QStringLiteral("Chat del proyecto · %1").arg(QDir(m_projectPath).dirName()), header);
    m_subtitle->setObjectName(QStringLiteral("chatSubtitle"));
    titles->addWidget(m_title);
    titles->addWidget(m_subtitle);
    m_newChat = new QPushButton(QStringLiteral("＋  Nuevo chat"), header);
    m_newChat->setObjectName(QStringLiteral("chatHeaderButton"));
    m_newChat->setCursor(Qt::PointingHandCursor);
    m_newChat->setToolTip(QStringLiteral("Empieza otra conversación con el contexto del proyecto."));
    m_headerRow->addWidget(m_avatar);
    m_headerRow->addLayout(titles);
    m_headerRow->addStretch(1);
    m_headerRow->addWidget(m_newChat);
    headerColumn->addLayout(m_headerRow);
    m_usage = new ChatUsageBar(header);
    headerColumn->addWidget(m_usage);
    layout->addWidget(header);

    m_view = new AgentChatView(this);
    m_view->setIdentity(m_identity);
    layout->addWidget(m_view, 1);

    auto *footer = new QWidget(this);
    footer->setObjectName(QStringLiteral("chatFooter"));
    auto *footerLayout = new QVBoxLayout(footer);
    footerLayout->setContentsMargins(16, 6, 16, 14);
    m_composer = new AgentChatComposer(footer);
    m_settings = new AgentChatSettings(m_projectPath, m_agentId, m_cliProgram, m_composer);
    m_mode = new ChatPillPicker(m_composer);
    m_mode->addItem(QStringLiteral("🔒 Solo lectura"), QStringLiteral("read-only"));
    m_mode->addItem(QStringLiteral("✎ Editar workspace"), QStringLiteral("workspace-write"));
    m_mode->setToolTip(QStringLiteral(
        "Solo lectura: el agente puede leer y proponer, sin modificar archivos.\n"
        "Editar workspace: puede modificar archivos del proyecto (sandbox de Codex / acceptEdits de Claude)."));
    m_composer->addToolbarWidget(m_settings);
    m_composer->addToolbarWidget(m_mode);
    footerLayout->addWidget(m_composer);
    layout->addWidget(footer);

    connect(m_composer, &AgentChatComposer::sendRequested, this, &AgentChatWidget::sendMessage);
    connect(m_composer, &AgentChatComposer::stopRequested, this, &AgentChatWidget::stop);
    connect(m_view, &AgentChatView::suggestionChosen, m_composer, &AgentChatComposer::setText);
    connect(m_newChat, &QPushButton::clicked, this, &AgentChatWidget::startNewChat);
    connect(m_settings, &AgentChatSettings::availabilityChanged, this, &AgentChatWidget::setAvailability);
    connect(m_settings, &AgentChatSettings::providerChanged, this, &AgentChatWidget::setProvider);

    m_process.setWorkingDirectory(m_projectPath);
    m_process.setProcessChannelMode(QProcess::SeparateChannels);
    connect(&m_process, &QProcess::readyReadStandardOutput, this, &AgentChatWidget::readOutput);
    connect(&m_process, qOverload<int, QProcess::ExitStatus>(&QProcess::finished),
        this, &AgentChatWidget::finishTurn);
    connect(&m_process, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (error != QProcess::FailedToStart) return;
        m_model.apply(localEvent(QStringLiteral("error"),
            QStringLiteral("No se pudo iniciar chxchx-tech: %1").arg(m_process.errorString())));
        setRunning(false);
        render();
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

bool AgentChatWidget::isAvailable() const
{
    return m_available;
}

void AgentChatWidget::focusInput()
{
    m_composer->focusInput();
}

void AgentChatWidget::sendMessage(const QString &text, const QStringList &images)
{
    const QString message = text.trimmed();
    if (message.isEmpty() || isRunning() || !m_available) return;
    if (m_cliProgram.isEmpty()) {
        m_composer->setHint(QStringLiteral("No se encontró chxchx-tech. Configura CHXCHX_TECH_CLI o añádelo al PATH."));
        return;
    }
    const bool firstTurn = m_model.sessionId().isEmpty();
    m_model.addUserMessage(message, images);
    m_composer->clear();
    QStringList arguments = {QStringLiteral("agent"), QStringLiteral("chat"), m_agentId,
        QStringLiteral("--path"), m_projectPath, QStringLiteral("--prompt"), message,
        QStringLiteral("--mode"), m_mode->currentData().toString(), QStringLiteral("--cancel-on-stdin")};
    if (!firstTurn) arguments << QStringLiteral("--session") << m_model.sessionId();
    if (!m_settings->model().isEmpty()) arguments << QStringLiteral("--model") << m_settings->model();
    if (!m_settings->effort().isEmpty()) arguments << QStringLiteral("--effort") << m_settings->effort();
    for (const QString &image : images) arguments << QStringLiteral("--image") << image;
    m_pending.clear();
    m_process.start(m_cliProgram, arguments);
    setRunning(true);
    m_view->setBusy(true, firstTurn ? QStringLiteral("Preparando el contexto del proyecto")
                                    : QStringLiteral("%1 está trabajando").arg(m_identity.name));
    render();
}

void AgentChatWidget::stop()
{
    if (!isRunning()) return;
    m_process.write("cancel\n");
    m_view->setBusy(true, QStringLiteral("Deteniendo a %1").arg(m_identity.name));
}

void AgentChatWidget::startNewChat()
{
    if (isRunning()) return;
    m_model.clear();
    render();
    focusInput();
}

void AgentChatWidget::setAvailability(bool available, const QString &installHint)
{
    m_available = available;
    m_composer->setInputEnabled(available, available ? QString()
        : QStringLiteral("%1 no está disponible todavía. Revisa el aviso de arriba.").arg(m_identity.name));
    if (available) return;
    const bool isCommand = installHint.startsWith(QStringLiteral("curl")) || installHint.startsWith(QStringLiteral("npm"));
    m_model.apply(localEvent(QStringLiteral("error"), isCommand
        ? QStringLiteral("%1 no está instalado o no está en el PATH. Instálalo con este comando y luego vuelve a abrir el chat:\n%2")
              .arg(m_identity.name, installHint)
        : installHint));
    render();
}

void AgentChatWidget::setProvider(const QString &provider, const QString &defaultModel)
{
    m_identity = chatIdentityFor(provider, m_agentId);
    QLabel *avatar = createChatAvatar(m_identity, 32, m_avatar->parentWidget());
    m_headerRow->replaceWidget(m_avatar, avatar);
    delete m_avatar;
    m_avatar = avatar;
    m_title->setText(m_identity.name);
    const QString project = QDir(m_projectPath).dirName();
    m_subtitle->setText(defaultModel.isEmpty() ? QStringLiteral("Chat del proyecto · %1").arg(project)
                                               : QStringLiteral("Chat del proyecto · %1 · %2").arg(project, defaultModel));
    m_view->setIdentity(m_identity);
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
    if (changed) render();
}

void AgentChatWidget::finishTurn(int exitCode, QProcess::ExitStatus status)
{
    readOutput();
    const QString stderrText = QString::fromUtf8(m_process.readAllStandardError()).trimmed();
    const bool reported = !m_model.entries().isEmpty()
        && m_model.entries().constLast().kind == AgentChatModel::Kind::Error;
    if ((status != QProcess::NormalExit || exitCode != 0) && !reported && !stderrText.isEmpty())
        m_model.apply(localEvent(QStringLiteral("error"), stderrText.right(2000)));
    setRunning(false);
    render();
    focusInput();
}

void AgentChatWidget::render()
{
    m_view->sync(m_model);
    m_usage->setUsage(m_model.usage(), m_model.usageSummary());
}

void AgentChatWidget::setRunning(bool running)
{
    m_composer->setRunning(running);
    m_newChat->setEnabled(!running);
    if (!running) m_view->setBusy(false);
    emit runningStateChanged(running);
}
