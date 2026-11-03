#include "git_repository.hpp"

#include "git_parsers.hpp"

#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QProcess>
#include <QProcessEnvironment>
#include <QTimer>

namespace {

constexpr int CommandTimeoutMs = 120000;
constexpr int KeptBackups = 20;
constexpr qint64 MaxBackupBytes = 64 * 1024 * 1024;

QByteArray nulJoined(const QStringList &paths)
{
    QByteArray joined;
    for (const QString &path : paths) {
        joined += path.toUtf8();
        joined.append('\0');
    }
    return joined;
}

bool mentions(const QString &text, std::initializer_list<const char *> needles)
{
    for (const char *needle : needles) if (text.contains(QLatin1String(needle), Qt::CaseInsensitive)) return true;
    return false;
}

QString friendlyMessage(GitProblem problem, const QString &stderrText, int exitCode)
{
    const QString text = stderrText.trimmed();
    switch (problem) {
    case GitProblem::Identity:
        return QStringLiteral("Git no sabe quién eres. Configura tu nombre y correo:\n"
                              "git config --global user.name \"Tu nombre\"\ngit config --global user.email \"tu@correo\"");
    case GitProblem::Credentials:
        return QStringLiteral("El remoto pide tus credenciales y Studio no las solicita por seguridad.\n"
                              "Configura un gestor de credenciales (git config --global credential.helper) o una llave SSH, "
                              "o ejecuta el comando una vez en la terminal (por ejemplo: git push) para que quede guardado.");
    case GitProblem::Rejected:
        return QStringLiteral("El remoto tiene cambios que tú no tienes, así que no acepta el envío.\n"
                              "Pulsa Recibir primero y vuelve a enviar.");
    case GitProblem::Diverged:
        return QStringLiteral("Tu rama y la del remoto avanzaron por separado: no se pueden unir en línea recta.");
    case GitProblem::NoRemote:
        return QStringLiteral("Este repositorio no tiene un remoto (por ejemplo origin).\n"
                              "Añade uno con: git remote add origin <url>");
    case GitProblem::Cancelled:
        return QStringLiteral("Cancelado.");
    default:
        break;
    }
    if (text.isEmpty()) return QStringLiteral("Git terminó con código %1.").arg(exitCode);
    return text;
}

} // namespace

QString gitBackupFolderName()
{
    return QStringLiteral("chxchx-backups");
}

GitProblem GitResult::problem() const
{
    if (ok()) return GitProblem::None;
    if (cancelled) return GitProblem::Cancelled;
    if (mentions(err, {"Please tell me who you are", "unable to auto-detect email", "empty ident name"})) return GitProblem::Identity;
    if (mentions(err, {"Authentication failed", "could not read Username", "could not read Password", "terminal prompts disabled",
                       "Permission denied (publickey", "Host key verification failed", "Invalid username or password"})) {
        return GitProblem::Credentials;
    }
    if (mentions(err, {"no upstream branch", "has no upstream", "There is no tracking information"})) return GitProblem::NoUpstream;
    if (mentions(err, {"No configured push destination", "No remote repository specified", "does not appear to be a git repository"})
        && !mentions(err, {"Permission denied"})) return GitProblem::NoRemote;
    if (mentions(err, {"non-fast-forward", "fetch first", "[rejected]", "Updates were rejected"})) return GitProblem::Rejected;
    if (mentions(err, {"Not possible to fast-forward", "divergent branches", "need to specify how to reconcile"})) return GitProblem::Diverged;
    return GitProblem::Other;
}

QString GitResult::message() const
{
    if (startFailed) return QStringLiteral("No se encontró Git o no pudo iniciarse. Instálalo desde https://git-scm.com.");
    if (truncated) return QStringLiteral("La salida de Git es demasiado grande y se recortó.");
    return friendlyMessage(problem(), err, exitCode);
}

GitRepository::GitRepository(QString workingDirectory, QObject *parent)
    : QObject(parent), m_directory(std::move(workingDirectory)), m_root(m_directory)
{
}

GitRepository::~GitRepository()
{
    // Nothing may call back into the views while they are being destroyed with us.
    m_queue.clear();
    if (m_process != nullptr) {
        m_process->disconnect();
        m_process->kill();
        m_process->waitForFinished(1000);
    }
}

bool GitRepository::isBusy() const
{
    return m_running || !m_queue.isEmpty();
}

void GitRepository::run(const QStringList &args, Callback done, const QByteArray &input, bool readOnly, qint64 maxOutput, int timeoutMs)
{
    m_queue.enqueue({args, std::move(done), input, readOnly, maxOutput, timeoutMs});
    if (!m_running) next();
}

void GitRepository::cancelCurrent()
{
    if (!m_running) return;
    if (!m_currentCancellable) {
        m_cancelNext = true;
        return;
    }
    m_cancelRequested = true;
    if (m_process != nullptr) m_process->kill();
}

void GitRepository::next()
{
    if (m_running || m_queue.isEmpty()) return;
    const Job job = m_queue.dequeue();
    if (m_cancelNext && job.timeoutMs > 0) {
        // The user cancelled while a quick command was running: the slow one never starts.
        m_cancelNext = false;
        GitResult cancelled;
        cancelled.cancelled = true;
        cancelled.exitCode = 0;
        if (job.done) job.done(cancelled);
        return next();
    }
    m_running = true;
    m_currentCancellable = job.timeoutMs > 0;
    auto *process = new QProcess(this);
    m_process = process;
    process->setWorkingDirectory(m_root);
    QProcessEnvironment environment = QProcessEnvironment::systemEnvironment();
    environment.insert(QStringLiteral("GIT_TERMINAL_PROMPT"), QStringLiteral("0"));
    environment.insert(QStringLiteral("GIT_PAGER"), QStringLiteral("cat"));
    environment.insert(QStringLiteral("GCM_INTERACTIVE"), QStringLiteral("never"));
    process->setProcessEnvironment(environment);
#ifdef Q_OS_UNIX
    // Without a controlling terminal, ssh and credential helpers cannot stop to ask for a password: they fail fast instead.
    process->setUnixProcessParameters(QProcess::UnixProcessFlag::CreateNewSession);
#endif
    QStringList arguments;
    if (job.readOnly) arguments << QStringLiteral("--no-optional-locks");
    arguments << QStringLiteral("-c") << QStringLiteral("core.quotepath=off") << job.args;

    auto result = std::make_shared<GitResult>();
    auto timeout = new QTimer(process);
    timeout->setSingleShot(true);
    const qint64 limit = job.maxOutput;
    connect(process, &QProcess::readyReadStandardOutput, process, [process, result, limit] {
        result->out += process->readAllStandardOutput();
        if (result->out.size() > limit) {
            result->out.truncate(limit);
            result->truncated = true;
            process->kill();
        }
    });
    connect(timeout, &QTimer::timeout, process, [process, result] {
        result->err = QStringLiteral("Git tardó demasiado y se detuvo.");
        process->kill();
    });
    const auto finish = [this, process, result, job](int exitCode) {
        result->cancelled = m_cancelRequested;
        m_cancelRequested = false;
        result->out += process->readAllStandardOutput();
        if (result->out.size() > job.maxOutput) {
            result->out.truncate(job.maxOutput);
            result->truncated = true;
        }
        const QString error = QString::fromLocal8Bit(process->readAllStandardError());
        if (!error.isEmpty()) result->err = error;
        result->exitCode = result->truncated ? 0 : exitCode;
        process->deleteLater();
        m_process = nullptr;
        m_running = false;
        m_currentCancellable = false;
        if (job.done) job.done(*result);
        if (!job.readOnly) emit changed();
        next();
    };
    connect(process, qOverload<int, QProcess::ExitStatus>(&QProcess::finished), process,
        [finish](int exitCode, QProcess::ExitStatus status) { finish(status == QProcess::NormalExit ? exitCode : -1); });
    connect(process, &QProcess::errorOccurred, process, [result, finish, process](QProcess::ProcessError error) {
        if (error != QProcess::FailedToStart) return;
        result->startFailed = true;
        finish(-1);
        Q_UNUSED(process);
    });
    process->start(QStringLiteral("git"), arguments);
    if (!job.input.isEmpty()) process->write(job.input);
    process->closeWriteChannel();
    timeout->start(job.timeoutMs > 0 ? job.timeoutMs : CommandTimeoutMs);
}

void GitRepository::withPaths(const QStringList &base, const QStringList &paths, Callback done)
{
    // Pathspecs travel on stdin so a large selection never hits the command-line size limit.
    run(base + QStringList{QStringLiteral("--pathspec-from-file=-"), QStringLiteral("--pathspec-file-nul")},
        std::move(done), nulJoined(paths));
}

void GitRepository::detect(const std::function<void(bool)> &done)
{
    run({QStringLiteral("rev-parse"), QStringLiteral("--show-toplevel")}, [this, done](const GitResult &result) {
        if (result.ok()) {
            const QString top = QString::fromUtf8(result.out).trimmed();
            if (!top.isEmpty()) m_root = QDir::cleanPath(top);
        }
        done(result.ok());
    }, {}, true);
}

void GitRepository::init(Callback done)
{
    run({QStringLiteral("init")}, std::move(done));
}

void GitRepository::status(const std::function<void(const GitResult &, const GitStatus &)> &done)
{
    run({QStringLiteral("status"), QStringLiteral("--porcelain=v2"), QStringLiteral("--branch"), QStringLiteral("-z"),
         QStringLiteral("--untracked-files=all")},
        [done](const GitResult &result) { done(result, result.ok() ? parseStatusV2(result.out) : GitStatus()); }, {}, true);
}

void GitRepository::stage(const QStringList &paths, Callback done)
{
    withPaths({QStringLiteral("add")}, paths, std::move(done));
}

void GitRepository::unstage(const QStringList &paths, bool initialCommit, Callback done)
{
    // Before the first commit there is no HEAD to restore from: drop the paths from the index instead.
    if (initialCommit) withPaths({QStringLiteral("rm"), QStringLiteral("--cached"), QStringLiteral("-r"), QStringLiteral("-q")}, paths, std::move(done));
    else withPaths({QStringLiteral("restore"), QStringLiteral("--staged")}, paths, std::move(done));
}

void GitRepository::discard(const QList<GitEntry> &entries, bool initialCommit,
                            const std::function<void(const GitResult &, const QString &)> &done)
{
    QStringList untracked;
    QStringList added;    // new in the index: removing the change means removing the file
    QStringList restore;  // tracked in HEAD: put the committed version back
    for (const GitEntry &entry : entries) {
        if (entry.untracked()) {
            untracked << entry.path;
        } else if (entry.kind == GitEntry::Kind::Renamed) {
            added << entry.path;
            restore << entry.origPath;
        } else if (entry.index == QLatin1Char('A')) {
            added << entry.path;
        } else {
            restore << entry.path;
        }
    }
    const bool tracked = !added.isEmpty() || !restore.isEmpty();
    const QString root = m_root;

    // Everything below runs after the backup exists, so a failed backup stops the discard.
    const auto apply = [this, untracked, added, restore, root, done, initialCommit](const QString &backup) {
        const auto finishWith = [done, backup](const GitResult &result) { done(result, backup); };
        const auto removeUntracked = [root, untracked, finishWith](const GitResult &previous) {
            if (!previous.ok()) return finishWith(previous);
            for (const QString &path : untracked) QFile::remove(QDir(root).filePath(path));
            finishWith(previous);
        };
        const auto restoreStep = [this, restore, removeUntracked, initialCommit](const GitResult &previous) {
            if (!previous.ok()) return removeUntracked(previous);
            if (restore.isEmpty()) return removeUntracked(previous);
            Q_UNUSED(initialCommit);
            withPaths({QStringLiteral("restore"), QStringLiteral("--source=HEAD"), QStringLiteral("--staged"), QStringLiteral("--worktree")},
                      restore, removeUntracked);
        };
        if (added.isEmpty()) return restoreStep(GitResult{0, {}, {}, false, false});
        withPaths({QStringLiteral("rm"), QStringLiteral("-f"), QStringLiteral("-q")}, added, restoreStep);
    };

    run({QStringLiteral("rev-parse"), QStringLiteral("--absolute-git-dir")}, [this, tracked, untracked, root, initialCommit, apply, done](const GitResult &dir) {
        if (!dir.ok()) return done(dir, QString());
        const QString stamp = QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMdd-HHmmss-zzz"));
        const QString backupRoot = QDir(QString::fromUtf8(dir.out).trimmed()).filePath(gitBackupFolderName());
        const QString backup = QDir(backupRoot).filePath(stamp);
        QDir().mkpath(backup);
        for (const QString &path : untracked) {
            const QString target = QDir(backup).filePath(QStringLiteral("untracked/") + path);
            QDir().mkpath(QFileInfo(target).absolutePath());
            QFile::copy(QDir(root).filePath(path), target);
        }
        const auto prune = [backupRoot] {
            QDir folder(backupRoot);
            const QStringList old = folder.entryList(QDir::Dirs | QDir::NoDotAndDotDot, QDir::Name);
            for (int i = 0; i < old.size() - KeptBackups; ++i) QDir(folder.filePath(old.at(i))).removeRecursively();
        };
        if (!tracked) {
            prune();
            return apply(backup);
        }
        // The whole diff against HEAD (prepared and not prepared) is a superset of what will be discarded.
        const QStringList diff = initialCommit
            ? QStringList{QStringLiteral("diff"), QStringLiteral("--cached"), QStringLiteral("--binary"), QStringLiteral("--no-color")}
            : QStringList{QStringLiteral("diff"), QStringLiteral("HEAD"), QStringLiteral("--binary"), QStringLiteral("--no-color")};
        run(diff, [backup, prune, apply, done](const GitResult &patch) {
            if (!patch.ok() || patch.truncated || patch.out.size() > MaxBackupBytes) {
                GitResult failed = patch;
                if (patch.ok()) failed.exitCode = 1;
                failed.err = QStringLiteral("No pude guardar una copia de seguridad de los cambios, así que no descarté nada.");
                return done(failed, QString());
            }
            QFile file(QDir(backup).filePath(QStringLiteral("changes.patch")));
            if (!file.open(QIODevice::WriteOnly) || file.write(patch.out) != patch.out.size()) {
                GitResult failed;
                failed.exitCode = 1;
                failed.err = QStringLiteral("No pude escribir la copia de seguridad en .git, así que no descarté nada.");
                return done(failed, QString());
            }
            file.close();
            prune();
            apply(backup);
        }, {}, true, MaxBackupBytes + 1);
    }, {}, true);
}

void GitRepository::commit(const QString &message, bool stageAll, Callback done)
{
    const QStringList commitArgs = {QStringLiteral("commit"), QStringLiteral("-F"), QStringLiteral("-")};
    const QByteArray text = message.toUtf8();
    if (!stageAll) return run(commitArgs, std::move(done), text);
    run({QStringLiteral("add"), QStringLiteral("-A")}, [this, commitArgs, text, done](const GitResult &added) {
        if (!added.ok()) return done(added);
        run(commitArgs, done, text);
    });
}

void GitRepository::branches(const std::function<void(const GitResult &, const QList<GitBranch> &)> &done)
{
    run({QStringLiteral("for-each-ref"), QString::fromLatin1(branchFormat()), QStringLiteral("--sort=-committerdate"),
         QStringLiteral("refs/heads"), QStringLiteral("refs/remotes")},
        [done](const GitResult &result) { done(result, result.ok() ? parseBranches(result.out) : QList<GitBranch>()); }, {}, true);
}

void GitRepository::switchBranch(const QString &name, bool remote, Callback done)
{
    if (remote) return run({QStringLiteral("switch"), QStringLiteral("--track"), name}, std::move(done));
    run({QStringLiteral("switch"), name}, std::move(done));
}

void GitRepository::createBranch(const QString &name, Callback done, const QString &startPoint)
{
    QStringList args = {QStringLiteral("switch"), QStringLiteral("-c"), name};
    if (!startPoint.isEmpty()) args << startPoint;
    run(args, std::move(done));
}

void GitRepository::deleteBranch(const QString &name, bool force, Callback done)
{
    run({QStringLiteral("branch"), force ? QStringLiteral("-D") : QStringLiteral("-d"), name}, std::move(done));
}

void GitRepository::log(int skip, int count, const std::function<void(const GitResult &, const QList<GitCommit> &)> &done)
{
    run({QStringLiteral("log"), QStringLiteral("-z"), QStringLiteral("--date-order"), QStringLiteral("--branches"),
         QStringLiteral("--remotes"), QStringLiteral("--tags"), QStringLiteral("HEAD"),
         QStringLiteral("--skip=%1").arg(skip), QStringLiteral("-n"), QString::number(count), QString::fromLatin1(logFormat())},
        [done](const GitResult &result) { done(result, result.ok() ? parseLog(result.out) : QList<GitCommit>()); }, {}, true);
}

void GitRepository::commitFiles(const QString &hash, const std::function<void(const GitResult &, const QList<GitFileChange> &)> &done)
{
    // `-m --first-parent` makes a merge commit show what it brought in, instead of an empty combined diff.
    run({QStringLiteral("show"), QStringLiteral("--format="), QStringLiteral("--name-status"), QStringLiteral("-z"),
         QStringLiteral("--find-renames"), QStringLiteral("-m"), QStringLiteral("--first-parent"), hash},
        [done](const GitResult &result) { done(result, result.ok() ? parseNameStatus(result.out) : QList<GitFileChange>()); }, {}, true);
}

void GitRepository::fileDiff(const QString &path, bool staged, const std::function<void(const GitResult &, const GitDiff &)> &done)
{
    QStringList args = {QStringLiteral("diff"), QStringLiteral("--no-ext-diff"), QStringLiteral("--no-color"), QStringLiteral("-U3")};
    if (staged) args << QStringLiteral("--cached");
    args << QStringLiteral("--") << path;
    run(args, [done](const GitResult &result) { done(result, result.ok() ? parseUnifiedDiff(result.out) : GitDiff()); }, {}, true);
}

void GitRepository::commitDiff(const QString &hash, const QString &path, const std::function<void(const GitResult &, const GitDiff &)> &done)
{
    run({QStringLiteral("show"), QStringLiteral("--no-ext-diff"), QStringLiteral("--no-color"), QStringLiteral("-U3"),
         QStringLiteral("-m"), QStringLiteral("--first-parent"), QStringLiteral("--format="), hash, QStringLiteral("--"), path},
        [done](const GitResult &result) { done(result, result.ok() ? parseUnifiedDiff(result.out) : GitDiff()); }, {}, true);
}

void GitRepository::headDiff(const QString &path, const std::function<void(const GitResult &, const GitDiff &)> &done)
{
    run({QStringLiteral("diff"), QStringLiteral("--no-ext-diff"), QStringLiteral("--no-color"), QStringLiteral("-U3"),
         QStringLiteral("HEAD"), QStringLiteral("--"), path},
        [done](const GitResult &result) { done(result, result.ok() ? parseUnifiedDiff(result.out) : GitDiff()); }, {}, true);
}

void GitRepository::commitMessage(const QString &hash, const std::function<void(const GitResult &, const QString &)> &done)
{
    run({QStringLiteral("show"), QStringLiteral("-s"), QStringLiteral("--format=%B"), hash},
        [done](const GitResult &result) { done(result, QString::fromUtf8(result.out).trimmed()); }, {}, true);
}
