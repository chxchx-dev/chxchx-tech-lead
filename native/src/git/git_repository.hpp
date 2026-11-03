#pragma once

#include "git_types.hpp"

#include <QObject>
#include <QQueue>

#include <functional>

class QProcess;

// What went wrong, in terms the interface can act on.
enum class GitProblem {
    None,
    Cancelled,
    Identity,     // git does not know the user's name and email
    Credentials,  // the remote needs a login Studio does not ask for
    Rejected,     // the remote has commits we do not have
    Diverged,     // pull --ff-only cannot fast-forward
    NoUpstream,   // the branch is not tracking a remote branch
    NoRemote,     // the repository has no remote at all
    Other
};

struct GitResult {
    int exitCode = -1;
    QByteArray out;
    QString err;
    bool startFailed = false;
    bool truncated = false;
    bool cancelled = false;
    bool ok() const { return !startFailed && !cancelled && exitCode == 0; }
    GitProblem problem() const;
    // What to tell the user when `ok()` is false.
    QString message() const;
};

enum class GitPullMode { FastForwardOnly, Merge };

// Runs `git` for one repository. Commands are serialized (so they never fight over
// `index.lock`) and asynchronous, so the interface stays responsive. Parsing lives in
// git_parsers; this class only builds commands, runs them and reports.
class GitRepository final : public QObject {
    Q_OBJECT
public:
    using Callback = std::function<void(const GitResult &)>;

    explicit GitRepository(QString workingDirectory, QObject *parent = nullptr);
    ~GitRepository() override;

    QString root() const { return m_root; }
    bool isBusy() const;

    // Low level. `readOnly` commands run with --no-optional-locks so they never block the user's own git.
    void run(const QStringList &args, Callback done, const QByteArray &input = {}, bool readOnly = false,
             qint64 maxOutput = 8 * 1024 * 1024, int timeoutMs = 0);
    // Stops the slow command that is running now (a download or upload); its callback sees `cancelled`.
    // Quick commands are never cancelled: if one is running, the next slow command is cancelled instead.
    void cancelCurrent();

    void detect(const std::function<void(bool isRepository)> &done);
    void init(Callback done);
    void status(const std::function<void(const GitResult &, const GitStatus &)> &done);
    void stage(const QStringList &paths, Callback done);
    void unstage(const QStringList &paths, bool initialCommit, Callback done);
    // Saves a copy of what is about to be lost inside .git, then discards. `backup` is empty when nothing was saved.
    void discard(const QList<GitEntry> &entries, bool initialCommit,
                 const std::function<void(const GitResult &, const QString &backup)> &done);
    void commit(const QString &message, bool stageAll, Callback done);
    void branches(const std::function<void(const GitResult &, const QList<GitBranch> &)> &done);
    void switchBranch(const QString &name, bool remote, Callback done);
    void createBranch(const QString &name, Callback done, const QString &startPoint = QString());
    void deleteBranch(const QString &name, bool force, Callback done);
    void log(int skip, int count, const std::function<void(const GitResult &, const QList<GitCommit> &)> &done);
    void commitFiles(const QString &hash, const std::function<void(const GitResult &, const QList<GitFileChange> &)> &done);
    void fileDiff(const QString &path, bool staged, const std::function<void(const GitResult &, const GitDiff &)> &done);
    // Prepared and not prepared changes together, as `git diff HEAD`.
    void headDiff(const QString &path, const std::function<void(const GitResult &, const GitDiff &)> &done);
    void commitMessage(const QString &hash, const std::function<void(const GitResult &, const QString &)> &done);
    void commitDiff(const QString &hash, const QString &path, const std::function<void(const GitResult &, const GitDiff &)> &done);

    // Remotes. None of these ever asks for a password: credentials come from the user's own git setup.
    void remotes(const std::function<void(const GitResult &, const QStringList &)> &done);
    void fetch(Callback done);
    void pull(GitPullMode mode, Callback done);
    // With `setUpstream`, publishes `branch` to `remote` and starts tracking it.
    void push(const QString &remote, const QString &branch, bool setUpstream, Callback done);
    void mergeInProgress(const std::function<void(bool)> &done);
    void abortMerge(Callback done);

    // Stash: work set aside, to be applied later.
    void stashList(const std::function<void(const GitResult &, const QList<GitStash> &)> &done);
    void stashPush(const QString &message, bool includeUntracked, Callback done);
    void stashApply(int index, Callback done);
    void stashPop(int index, Callback done);
    void stashDrop(int index, Callback done);

signals:
    // After any command that may have altered the repository.
    void changed();

private:
    struct Job {
        QStringList args;
        Callback done;
        QByteArray input;
        bool readOnly = false;
        qint64 maxOutput = 0;
        int timeoutMs = 0;
    };
    void next();
    void withPaths(const QStringList &base, const QStringList &paths, Callback done);

    QString m_directory;
    QString m_root;
    QQueue<Job> m_queue;
    QProcess *m_process = nullptr;
    bool m_running = false;
    bool m_cancelRequested = false;
    bool m_cancelNext = false;
    bool m_currentCancellable = false;
};

// Folder, below `.git`, where discarded work is saved before it is removed.
QString gitBackupFolderName();
