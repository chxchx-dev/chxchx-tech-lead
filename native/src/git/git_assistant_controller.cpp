#include "git_assistant_controller.hpp"

#include "git_repository.hpp"

#include <QCheckBox>
#include <QDir>
#include <QFile>
#include <QMessageBox>
#include <QProcess>
#include <QSettings>
#include <QTimer>

namespace {

constexpr int MaxUntrackedFiles = 6;
constexpr qint64 MaxConflictFileBytes = 300 * 1024;
const char *AgentKey = "git/assistantAgent";

QString modelKey(const QString &agentId)
{
    return QStringLiteral("git/assistantModel/%1").arg(agentId);
}

QString consentKey(const QString &agentId)
{
    return QStringLiteral("git/assistantConsent/%1").arg(agentId);
}

} // namespace

GitAssistantController::GitAssistantController(GitRepository *repository, QString projectPath, QObject *parent)
    : QObject(parent), m_repository(repository), m_projectPath(std::move(projectPath))
{
    m_assistant = new GitAssistant(QString(), m_projectPath, this);
}

void GitAssistantController::configure(const QString &cliProgram, AgentProvider agents, PreferredProvider preferred)
{
    m_assistant->setCliProgram(cliProgram);
    m_cli = cliProgram;
    m_agents = std::move(agents);
    m_preferred = std::move(preferred);
    emit agentsChanged();
}

void GitAssistantController::setSnapshot(const GitStatus &status, bool mergeInProgress, int stashCount)
{
    m_status = status;
    m_mergeInProgress = mergeInProgress;
    m_stashCount = stashCount;
}

QList<GitAssistantAgent> GitAssistantController::agents() const
{
    return m_agents ? m_agents() : QList<GitAssistantAgent>();
}

QString GitAssistantController::currentAgentId() const
{
    const QList<GitAssistantAgent> list = agents();
    const auto has = [&list](const QString &id) {
        for (const GitAssistantAgent &agent : list) if (agent.id == id) return true;
        return false;
    };
    if (list.isEmpty()) return {};
    if (!m_chosen.isEmpty() && has(m_chosen)) return m_chosen;
    const QString preferred = m_preferred ? m_preferred() : QString();
    if (!preferred.isEmpty() && has(preferred)) return preferred;
    const QString saved = QSettings().value(QLatin1String(AgentKey)).toString();
    if (!saved.isEmpty() && has(saved)) return saved;
    return list.first().id;
}

QString GitAssistantController::currentAgentLabel() const
{
    const QString id = currentAgentId();
    for (const GitAssistantAgent &agent : agents()) if (agent.id == id) return agent.label;
    return id;
}

void GitAssistantController::chooseAgent(const QString &id)
{
    m_chosen = id;
    QSettings().setValue(QLatin1String(AgentKey), id);
    emit agentsChanged();
}

QString GitAssistantController::modelPreference(const QString &agentId) const
{
    return QSettings().value(modelKey(agentId)).toString();
}

void GitAssistantController::setModelPreference(const QString &preference)
{
    const QString agent = currentAgentId();
    if (agent.isEmpty()) return;
    if (preference.isEmpty()) QSettings().remove(modelKey(agent));
    else QSettings().setValue(modelKey(agent), preference);
    emit modelsChanged();
}

GitModelChoice GitAssistantController::currentChoice() const
{
    const QString agent = currentAgentId();
    return chooseModel(m_models.value(agent), modelPreference(agent));
}

QString GitAssistantController::labelWithModel() const
{
    const GitModelChoice choice = currentChoice();
    return choice.model.isEmpty() ? currentAgentLabel() : QStringLiteral("%1 · %2").arg(currentAgentLabel(), choice.model);
}

void GitAssistantController::refreshModels()
{
    const QString agent = currentAgentId();
    if (!agent.isEmpty()) ensureModels(agent, [] {});
}

void GitAssistantController::ensureModels(const QString &agentId, const std::function<void()> &then)
{
    if (m_fetched.contains(agentId) || m_cli.isEmpty()) return then();
    const bool alreadyFetching = m_waiting.contains(agentId);
    m_waiting[agentId].append(then);
    if (alreadyFetching) return;
    // The agent lists its own models; if it cannot (not installed, no trust), the agent's default is used.
    auto *process = new QProcess(this);
    auto *timeout = new QTimer(process);
    timeout->setSingleShot(true);
    connect(timeout, &QTimer::timeout, process, [process] { process->kill(); });
    connect(process, qOverload<int, QProcess::ExitStatus>(&QProcess::finished), this, [this, agentId, process](int code, QProcess::ExitStatus) {
        GitModelList list;
        if (code == 0) list = parseAgentOptions(process->readAllStandardOutput());
        process->deleteLater();
        m_models.insert(agentId, list);
        m_fetched.insert(agentId);
        const QList<std::function<void()>> waiting = m_waiting.take(agentId);
        emit modelsChanged();
        for (const auto &callback : waiting) callback();
    });
    connect(process, &QProcess::errorOccurred, process, [process](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart) emit process->finished(-1, QProcess::CrashExit);
    });
    process->start(m_cli, {QStringLiteral("bridge"), QStringLiteral("agent-options"), m_projectPath, QStringLiteral("--agent"), agentId});
    timeout->start(20000);
}

void GitAssistantController::setBusy(bool busy, const QString &text)
{
    m_busy = busy;
    emit busyChanged(busy, text);
}

bool GitAssistantController::begin(const QString &what, const QString &consentText)
{
    if (m_busy) return false;
    const QString agent = currentAgentId();
    if (agent.isEmpty()) {
        emit failed(QStringLiteral("Conecta un agente (Codex, Claude Code u OpenCode) en Agentes para recibir ayuda."));
        return false;
    }
    if (m_consentRequired && !QSettings().value(consentKey(agent), false).toBool()) {
        QMessageBox box;
        box.setWindowTitle(QStringLiteral("Enviar al asistente"));
        box.setIcon(QMessageBox::Question);
        box.setText(consentText.arg(currentAgentLabel()));
        box.setInformativeText(QStringLiteral("No se envían archivos sensibles (.env, llaves, credenciales) y se enmascaran valores que parezcan secretos. "
                                              "El agente solo puede leer: no modifica nada."));
        auto *again = new QCheckBox(QStringLiteral("No volver a preguntar con este agente"), &box);
        box.setCheckBox(again);
        box.setStandardButtons(QMessageBox::Ok | QMessageBox::Cancel);
        box.setDefaultButton(QMessageBox::Ok);
        if (box.exec() != QMessageBox::Ok) return false;
        if (again->isChecked()) QSettings().setValue(consentKey(agent), true);
    }
    setBusy(true, QStringLiteral("%1 está %2…").arg(currentAgentLabel(), what));
    return true;
}

void GitAssistantController::gather(bool wantDiff, bool wantConflicts, int recentCount, const std::function<void(const GitAssistContext &)> &done)
{
    auto context = std::make_shared<GitAssistContext>();
    context->status = m_status;
    context->mergeInProgress = m_mergeInProgress;
    context->stashCount = m_stashCount;
    if (wantConflicts) {
        for (const GitEntry &entry : m_status.conflicts()) {
            context->conflictFiles << entry.path;
            if (isSensitivePath(entry.path)) continue;
            QFile file(QDir(m_repository->root()).filePath(entry.path));
            if (file.size() > MaxConflictFileBytes || !file.open(QIODevice::ReadOnly)) continue;
            const QString hunks = extractConflictHunks(QString::fromUtf8(file.readAll()));
            if (!hunks.isEmpty()) context->conflictExcerpts += QStringLiteral("%1:\n%2\n\n").arg(entry.path, hunks);
        }
    }
    // Whether a merge is half done is asked now, not taken from an earlier look: the question must not be built on stale state.
    const auto proceed = [this, context, wantDiff, recentCount, done] {
        const auto finishWithLog = [this, context, recentCount, done] {
            m_repository->run({QStringLiteral("log"), QStringLiteral("-n"), QString::number(recentCount), QStringLiteral("--format=%s")},
                [context, done](const GitResult &result) {
                    if (result.ok()) context->recentSubjects = QString::fromUtf8(result.out).split(QLatin1Char('\n'), Qt::SkipEmptyParts);
                    done(*context);
                }, {}, true);
        };
        if (!wantDiff) return finishWithLog();
        const bool staged = !m_status.stagedEntries().isEmpty();
        QStringList diff = {QStringLiteral("diff"), QStringLiteral("--no-ext-diff"), QStringLiteral("--no-color"), QStringLiteral("-U2")};
        if (staged || m_status.initial) diff << QStringLiteral("--cached");
        else diff << QStringLiteral("HEAD");
        m_repository->run(diff, [this, context, staged, finishWithLog](const GitResult &result) {
            if (result.ok()) context->diff = QString::fromUtf8(result.out);
            if (!staged) {
                int taken = 0;
                for (const GitEntry &entry : m_status.changedEntries()) {
                    if (!entry.untracked() || isSensitivePath(entry.path) || taken >= MaxUntrackedFiles) continue;
                    QFile file(QDir(m_repository->root()).filePath(entry.path));
                    if (!file.open(QIODevice::ReadOnly)) continue;
                    const QByteArray bytes = file.read(8192);
                    if (bytes.contains('\0')) continue;
                    context->diff += newFileDiff(entry.path, QString::fromUtf8(bytes));
                    ++taken;
                }
            }
            finishWithLog();
        }, {}, true);
    };
    if (!wantConflicts) return proceed();
    m_repository->mergeInProgress([context, proceed](bool inProgress) {
        context->mergeInProgress = inProgress;
        proceed();
    });
}

void GitAssistantController::suggestCommitMessage()
{
    if (m_status.stagedEntries().isEmpty() && m_status.changedEntries().isEmpty()) {
        return emit failed(QStringLiteral("No hay cambios para describir."));
    }
    if (!begin(QStringLiteral("leyendo tus cambios"), QStringLiteral("Se enviará a %1 el resumen y las diferencias (diff) de tus cambios para redactar el mensaje del commit."))) return;
    const int generation = ++m_generation;
    gather(true, false, 10, [this, generation](const GitAssistContext &context) {
        if (generation != m_generation) return;
        const QString agent = currentAgentId();
        disconnect(m_assistant, &GitAssistant::finished, nullptr, nullptr);
        connect(m_assistant, &GitAssistant::finished, this, [this, generation](const GitAssistantResult &result) {
            if (generation != m_generation) return;
            setBusy(false, QString());
            if (result.cancelled) return emit failed(QStringLiteral("Cancelado."));
            const QString message = result.ok() ? cleanCommitMessage(result.text) : QString();
            if (message.isEmpty()) return emit failed(result.error.isEmpty() ? QStringLiteral("El asistente no devolvió un mensaje. Inténtalo otra vez.") : result.error);
            emit commitMessageReady(message, labelWithModel());
        }, Qt::SingleShotConnection);
        const QString prompt = buildCommitPrompt(context);
        ensureModels(agent, [this, generation, agent, prompt] {
            if (generation != m_generation) return;
            const GitModelChoice choice = currentChoice();
            setBusy(true, QStringLiteral("%1 está redactando el mensaje…").arg(labelWithModel()));
            m_assistant->ask(agent, prompt, choice.model, choice.effort);
        });
    });
}

void GitAssistantController::advise(GitAdviceKind kind, const QString &lastError)
{
    const bool conflicts = kind == GitAdviceKind::Conflicts || !m_status.conflicts().isEmpty();
    if (!begin(QStringLiteral("preparando el consejo"),
               QStringLiteral("Se enviará a %1 el estado de tu repositorio (rama, cambios pendientes, el último mensaje de Git%2) para aconsejarte.")
                   .arg(QStringLiteral("%1"), conflicts ? QStringLiteral(" y los fragmentos en conflicto") : QString()))) return;
    const int generation = ++m_generation;
    gather(false, conflicts, 8, [this, generation, kind, lastError](const GitAssistContext &base) {
        if (generation != m_generation) return;
        GitAssistContext context = base;
        context.lastError = lastError;
        disconnect(m_assistant, &GitAssistant::finished, nullptr, nullptr);
        connect(m_assistant, &GitAssistant::finished, this, [this, generation, kind](const GitAssistantResult &result) {
            if (generation != m_generation) return;
            setBusy(false, QString());
            if (result.cancelled) return emit failed(QStringLiteral("Cancelado."));
            if (!result.ok()) return emit failed(result.error.isEmpty() ? QStringLiteral("El asistente no devolvió una respuesta.") : result.error);
            emit adviceReady(kind, result.text, labelWithModel());
        }, Qt::SingleShotConnection);
        const QString prompt = buildAdvicePrompt(kind, context);
        const QString agent = currentAgentId();
        ensureModels(agent, [this, generation, agent, prompt] {
            if (generation != m_generation) return;
            const GitModelChoice choice = currentChoice();
            m_assistant->ask(agent, prompt, choice.model, choice.effort);
        });
    });
}

void GitAssistantController::cancel()
{
    if (!m_busy) return;
    ++m_generation;  // a late answer is dropped
    m_assistant->cancel();
    setBusy(false, QString());
}
