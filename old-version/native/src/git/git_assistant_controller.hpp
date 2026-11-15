#pragma once

#include "git_assistant.hpp"
#include "git_assistant_models.hpp"
#include "git_assistant_prompts.hpp"
#include "git_commit_plan.hpp"

#include <QHash>
#include <QObject>
#include <QSet>

#include <functional>

class GitRepository;

struct GitAssistantAgent {
    QString id;
    QString label;
};

// Connects the Git panel to an agent: which one to use, asking the person first, collecting only what the
// question needs from the repository, and turning the answer into a commit message or a piece of advice.
class GitAssistantController final : public QObject {
    Q_OBJECT
public:
    using AgentProvider = std::function<QList<GitAssistantAgent>()>;
    using PreferredProvider = std::function<QString()>;

    GitAssistantController(GitRepository *repository, QString projectPath, QObject *parent = nullptr);

    void configure(const QString &cliProgram, AgentProvider agents, PreferredProvider preferred);
    // Kept current by the panel: the question is built from this snapshot.
    void setSnapshot(const GitStatus &status, bool mergeInProgress, int stashCount);
    QList<GitAssistantAgent> agents() const;
    QString currentAgentId() const;
    QString currentAgentLabel() const;
    void chooseAgent(const QString &id);
    // Which model answers: automatic (a light one), the agent's own default, or one the person picked.
    GitModelList modelList(const QString &agentId) const { return m_models.value(agentId); }
    QString modelPreference(const QString &agentId) const;
    void setModelPreference(const QString &preference);
    GitModelChoice currentChoice() const;
    // Reads the current agent's model list once (it comes from the agent itself).
    void refreshModels();
    void suggestCommitMessage();
    void advise(GitAdviceKind kind, const QString &lastError = QString());
    void cancel();
    bool busy() const { return m_busy; }
    // Tests: skip the confirmation that precedes the first question to an agent.
    void setConsentRequired(bool required) { m_consentRequired = required; }

signals:
    void busyChanged(bool busy, const QString &text);
    void commitMessageReady(const QString &message, const QString &agentLabel);
    // The changes belong to several topics: one proposed commit for each.
    void commitPlanReady(const QList<GitCommitGroup> &plan, const QString &agentLabel);
    void adviceReady(GitAdviceKind kind, const QString &markdown, const QString &agentLabel);
    void failed(const QString &message);
    void agentsChanged();
    void modelsChanged();

private:
    bool begin(const QString &what, const QString &consentText);
    void gather(bool wantDiff, bool wantConflicts, int recentCount, const std::function<void(const GitAssistContext &)> &done);
    void setBusy(bool busy, const QString &text);
    void ensureModels(const QString &agentId, const std::function<void()> &then);
    QString labelWithModel() const;

    GitRepository *m_repository;
    QString m_projectPath;
    GitAssistant *m_assistant = nullptr;
    AgentProvider m_agents;
    PreferredProvider m_preferred;
    GitStatus m_status;
    bool m_mergeInProgress = false;
    int m_stashCount = 0;
    QString m_chosen;
    QString m_cli;
    QHash<QString, GitModelList> m_models;
    QSet<QString> m_fetched;
    QHash<QString, QList<std::function<void()>>> m_waiting;
    bool m_busy = false;
    bool m_consentRequired = true;
    int m_generation = 0;
};
