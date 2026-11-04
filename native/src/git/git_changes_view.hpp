#pragma once

#include "git_assistant_controller.hpp"
#include "git_diff_view.hpp"
#include "git_guide.hpp"
#include "git_types.hpp"

#include <QWidget>

class GitAssistantBar;
class GitRepository;
class QToolButton;
class QLabel;
class QPlainTextEdit;
class QPushButton;
class QTreeWidget;
class QTreeWidgetItem;

// The Changes tab: conflicts, what is ready to be saved, and what changed since; plus the commit box.
class GitChangesView final : public QWidget {
    Q_OBJECT
public:
    explicit GitChangesView(GitRepository *repository, QWidget *parent = nullptr);
    void setStatus(const GitStatus &status);
    void setStashes(const QList<GitStash> &stashes);
    void setMergeInProgress(bool inProgress);
    // The next-step card and the assistant.
    void setGuidance(const GitGuidance &guidance);
    void setAssistantAgents(const QList<GitAssistantAgent> &agents, const QString &currentId);
    void setAssistantBusy(bool busy, const QString &text);
    void setAssistantModels(const GitModelList &list, const QString &preference);
    void setAssistantNote(const QString &note);
    GitAssistantBar *assistantBar() const { return m_bar; }
    bool suggestEnabled() const;
    QString messageText() const;
    QString assistantNote() const;
    void requestSuggestion() { emit suggestMessageRequested(); }

    // For tests: act without dialogs.
    void setConfirmations(bool enabled) { m_confirm = enabled; }
    int sectionCount(const QString &section) const;
    QString commitButtonText() const;
    bool commitEnabled() const;
    void setMessage(const QString &message);
    void commitNow();
    void stagePaths(const QStringList &paths);
    void unstagePaths(const QStringList &paths);
    void discardPaths(const QStringList &paths);
    bool emptyStateVisible() const;
    void stashChanges(const QString &message, bool includeUntracked);
    void applyStash(int index);
    void popStash(int index);
    void dropStash(int index);
    void abortMerge();
    bool abortMergeVisible() const;

signals:
    void openDiffRequested(const GitDiffRequest &request);
    void openFileRequested(const QString &relativePath);
    void message(const QString &text, bool error);
    void suggestMessageRequested();
    void adviceRequested(GitAdviceKind kind);
    void assistantCancelRequested();
    void assistantAgentChosen(const QString &id);
    void assistantModelChosen(const QString &preference);

private:
    void rebuild();
    void askStash();
    void restyle();
    void updateCommitArea();
    QTreeWidgetItem *addSection(const QString &key, const QString &title, int count);
    QWidget *sectionHeader(const QString &key, const QString &title, int count);
    QList<GitEntry> selectedEntries() const;
    void contextMenu(const QPoint &position);
    void openDiffFor(const QModelIndex &index);
    const GitEntry *entryFor(const QString &section, const QString &path) const;

    GitRepository *m_repository;
    GitStatus m_status;
    bool m_confirm = true;
    QPlainTextEdit *m_message = nullptr;
    QLabel *m_hint = nullptr;
    QPushButton *m_commit = nullptr;
    QToolButton *m_suggest = nullptr;
    QLabel *m_note = nullptr;
    GitAssistantBar *m_bar = nullptr;
    bool m_hasAgents = false;
    bool m_assistantBusy = false;
    QLabel *m_banner = nullptr;
    QWidget *m_emptyBox = nullptr;
    QLabel *m_emptyIcon = nullptr;
    QPushButton *m_abortMerge = nullptr;
    QList<GitStash> m_stashes;
    bool m_mergeInProgress = false;
    QLabel *m_empty = nullptr;
    QTreeWidget *m_tree = nullptr;
};
