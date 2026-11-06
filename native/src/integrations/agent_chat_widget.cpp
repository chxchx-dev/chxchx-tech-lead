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
#include <QMessageBox>
#include <QPushButton>
#include <QStyle>
#include <QToolButton>
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
    if (id.contains(QStringLiteral("opencode"))) return QStringLiteral("OpenCode");
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
    m_mode->addItem(QStringLiteral("✎ Editar archivos"), QStringLiteral("workspace-write"));
    m_mode->addItem(QStringLiteral("⚡ Editar y ejecutar"), QStringLiteral("workspace-commands"));
    m_mode->addItem(QStringLiteral("⚠ Acceso total"), QStringLiteral("full-access"));
    m_mode->setCurrentIndex(1); // the same default as before: edit the project, commands ask first
    m_lastMode = 1;
    m_mode->setToolTip(QStringLiteral(
        "Qué puede hacer el agente en el próximo mensaje:\n"
        "🔒 Solo lectura: lee y propone, sin modificar nada.\n"
        "✎ Editar archivos: modifica el proyecto; los comandos y la web piden tu permiso.\n"
        "⚡ Editar y ejecutar: además ejecuta comandos sin preguntar (Codex los corre dentro de su sandbox).\n"
        "⚠ Acceso total: sin restricciones ni sandbox. Úsalo solo en proyectos de confianza."));
    m_network = new QToolButton(m_composer);
    m_network->setObjectName(QStringLiteral("pillToggle"));
    m_network->setText(QStringLiteral("🌐 Red"));
    m_network->setCheckable(true);
    m_network->setCursor(Qt::PointingHandCursor);
    m_network->setToolTip(QStringLiteral(
        "Permite que el agente use internet (descargar, consultar páginas). Apagado, lo que necesite red te pedirá permiso."));
    connect(m_mode, &ChatPillPicker::currentIndexChanged, this, [this](int index) {
        const bool full = m_mode->currentData().toString() == QStringLiteral("full-access");
        if (full && !confirmFullAccess()) {
            const QSignalBlocker block(m_mode);
            m_mode->setCurrentIndex(m_lastMode);
            return;
        }
        m_lastMode = index;
        m_mode->setProperty("level", m_mode->currentData().toString());
        m_mode->style()->unpolish(m_mode);
        m_mode->style()->polish(m_mode);
    });
    m_composer->addToolbarWidget(m_settings);
    m_composer->addToolbarWidget(m_mode);
    m_composer->addToolbarWidget(m_network);
    footerLayout->addWidget(m_composer);
    layout->addWidget(footer);

    connect(m_composer, &AgentChatComposer::sendRequested, this, &AgentChatWidget::sendMessage);
    connect(m_composer, &AgentChatComposer::stopRequested, this, &AgentChatWidget::stop);
    connect(m_view, &AgentChatView::suggestionChosen, m_composer, &AgentChatComposer::setText);
    connect(m_view, &AgentChatView::permissionAnswered, this, &AgentChatWidget::answerPermission);
    connect(m_view, &AgentChatView::questionAnswered, this, &AgentChatWidget::answerQuestion);
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
    return name == QStringLiteral("codex") || name == QStringLiteral("claude") || name == QStringLiteral("opencode");
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
        QStringLiteral("--cancel-on-stdin"), QStringLiteral("--live")};
    arguments << accessArguments();
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
    // Approvals belong to the conversation: a new chat starts from what the selectors show.
    m_approved.clear();
    m_onceAllow.clear();
    m_onceMode.clear();
    m_onceNetwork = false;
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
    if (changed) {
        render();
        awaitingAnswerChanged();
    }
}

// While Claude waits for an answer the typing indicator says so, instead of pretending it is working.
void AgentChatWidget::awaitingAnswerChanged()
{
    if (!isRunning()) return;
    m_view->setBusy(true, m_model.awaitingAnswer() ? QStringLiteral("%1 espera tu respuesta").arg(m_identity.name)
                                                   : QStringLiteral("%1 está trabajando").arg(m_identity.name));
}

void AgentChatWidget::writeAnswer(const QJsonObject &answer)
{
    if (!isRunning()) return;
    QJsonObject line = answer;
    line.insert(QStringLiteral("type"), QStringLiteral("answer"));
    m_process.write(QJsonDocument(line).toJson(QJsonDocument::Compact) + '\n');
}

void AgentChatWidget::finishTurn(int exitCode, QProcess::ExitStatus status)
{
    readOutput();
    const QString stderrText = QString::fromUtf8(m_process.readAllStandardError()).trimmed();
    const bool reported = !m_model.entries().isEmpty()
        && m_model.entries().constLast().kind == AgentChatModel::Kind::Error;
    if ((status != QProcess::NormalExit || exitCode != 0) && !reported && !stderrText.isEmpty())
        m_model.apply(localEvent(QStringLiteral("error"), stderrText.right(2000)));
    m_model.expireLiveRequests();
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

QString AgentChatWidget::accessMode() const
{
    return m_mode->currentData().toString();
}

void AgentChatWidget::setAccessMode(const QString &mode)
{
    const int index = m_mode->findData(mode);
    if (index >= 0) m_mode->setCurrentIndex(index);
}

bool AgentChatWidget::networkAllowed() const
{
    return m_network->isChecked();
}

bool AgentChatWidget::confirmFullAccess()
{
    if (m_confirmFullAccess) return m_confirmFullAccess();
    QMessageBox box(QMessageBox::Warning, QStringLiteral("Acceso total"),
        QStringLiteral("¿Dar a %1 acceso total?").arg(m_identity.name), QMessageBox::NoButton, this);
    box.setInformativeText(QStringLiteral(
        "Podrá modificar y borrar archivos fuera del proyecto, ejecutar cualquier comando y usar la red, "
        "sin pedirte permiso y sin sandbox. Hazlo solo en proyectos de confianza."));
    auto *allow = box.addButton(QStringLiteral("Dar acceso total"), QMessageBox::DestructiveRole);
    auto *cancel = box.addButton(QStringLiteral("Cancelar"), QMessageBox::RejectRole);
    box.setDefaultButton(cancel);
    box.exec();
    return box.clickedButton() == allow;
}

// What the next message may do: the selectors plus whatever the person just approved. One-off grants
// are spent here so a single "allow once" never becomes permanent.
QStringList AgentChatWidget::accessArguments()
{
    QStringList arguments = {QStringLiteral("--mode"), m_onceMode.isEmpty() ? accessMode() : m_onceMode};
    if (m_network->isChecked() || m_onceNetwork) arguments << QStringLiteral("--network");
    QStringList rules = m_approved;
    for (const QString &rule : std::as_const(m_onceAllow)) {
        if (!rules.contains(rule)) rules << rule;
    }
    for (const QString &rule : std::as_const(rules)) arguments << QStringLiteral("--allow") << rule;
    m_onceAllow.clear();
    m_onceMode.clear();
    m_onceNetwork = false;
    return arguments;
}

void AgentChatWidget::answerPermission(int entryIndex, int choice)
{
    const auto &entries = m_model.entries();
    if (entryIndex < 0 || entryIndex >= entries.size()) return;
    const auto &entry = entries.at(entryIndex);
    if (entry.kind != AgentChatModel::Kind::Permission || !entry.decision.isEmpty()) return;
    const bool live = !entry.requestId.isEmpty();
    if (live && !isRunning()) return; // the agent is no longer waiting
    if (choice < 0 || choice >= entry.choices.size()) {
        if (live) writeAnswer({{QStringLiteral("request_id"), entry.requestId}, {QStringLiteral("behavior"), QStringLiteral("deny")}});
        m_model.decidePermission(entryIndex, QString());
        render();
        if (live) awaitingAnswerChanged();
        return;
    }
    const QJsonObject picked = entry.choices.at(choice).toObject();
    const QString mode = picked.value(QStringLiteral("mode")).toString();
    const bool persist = picked.value(QStringLiteral("persist")).toBool();
    if (mode == QStringLiteral("full-access") && !confirmFullAccess()) return;
    QStringList rules;
    for (const auto &rule : picked.value(QStringLiteral("allow")).toArray()) rules << rule.toString();
    if (persist) {
        for (const QString &rule : std::as_const(rules)) {
            if (!m_approved.contains(rule)) m_approved << rule;
        }
        if (picked.value(QStringLiteral("network")).toBool()) m_network->setChecked(true);
        if (!mode.isEmpty()) setAccessMode(mode);
    } else {
        m_onceAllow << rules;
        if (picked.value(QStringLiteral("network")).toBool()) m_onceNetwork = true;
        if (!mode.isEmpty()) m_onceMode = mode;
    }
    m_model.decidePermission(entryIndex, picked.value(QStringLiteral("label")).toString());
    if (live) {
        // The agent is paused on this very request: answer it, no new message needed.
        writeAnswer({{QStringLiteral("request_id"), entry.requestId}, {QStringLiteral("behavior"), QStringLiteral("allow")}});
        render();
        awaitingAnswerChanged();
        return;
    }
    render();
    sendMessage(QStringLiteral("Ya tienes el permiso que faltaba. Vuelve a intentar lo que quedó bloqueado."));
}

void AgentChatWidget::answerQuestion(int entryIndex, const QJsonObject &answers)
{
    const auto &entries = m_model.entries();
    if (entryIndex < 0 || entryIndex >= entries.size() || !isRunning()) return;
    const auto &entry = entries.at(entryIndex);
    if (entry.kind != AgentChatModel::Kind::Question || !entry.decision.isEmpty() || entry.requestId.isEmpty()) return;
    QStringList summary;
    for (auto it = answers.begin(); it != answers.end(); ++it) summary << it.value().toString();
    writeAnswer({{QStringLiteral("request_id"), entry.requestId}, {QStringLiteral("answers"), answers}});
    m_model.decidePermission(entryIndex, summary.join(QStringLiteral(" · ")));
    render();
    awaitingAnswerChanged();
}
