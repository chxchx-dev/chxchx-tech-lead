// Remotes and stash: the parts of GitRepository that talk to other repositories or set work aside.
#include "git_repository.hpp"

#include "git_parsers.hpp"

namespace {

constexpr int NetworkTimeoutMs = 10 * 60 * 1000;

} // namespace

void GitRepository::remotes(const std::function<void(const GitResult &, const QStringList &)> &done)
{
    run({QStringLiteral("remote")}, [done](const GitResult &result) {
        QStringList names;
        if (result.ok()) for (const QString &line : QString::fromUtf8(result.out).split(QLatin1Char('\n'), Qt::SkipEmptyParts)) names << line.trimmed();
        done(result, names);
    }, {}, true);
}

void GitRepository::fetch(Callback done)
{
    run({QStringLiteral("fetch"), QStringLiteral("--all"), QStringLiteral("--prune")}, std::move(done), {}, false, 8 * 1024 * 1024, NetworkTimeoutMs);
}

void GitRepository::pull(GitPullMode mode, Callback done)
{
    // Fast-forward only is the safe default: it never creates a merge or touches your commits on its own.
    const QStringList args = mode == GitPullMode::FastForwardOnly
        ? QStringList{QStringLiteral("pull"), QStringLiteral("--ff-only")}
        : QStringList{QStringLiteral("pull"), QStringLiteral("--no-rebase"), QStringLiteral("--no-edit")};
    run(args, std::move(done), {}, false, 8 * 1024 * 1024, NetworkTimeoutMs);
}

void GitRepository::push(const QString &remote, const QString &branch, bool setUpstream, Callback done)
{
    // Never forced: a rejected push must tell the user to receive first, not overwrite the remote.
    QStringList args = {QStringLiteral("push")};
    if (setUpstream) args << QStringLiteral("-u") << remote << branch;
    run(args, std::move(done), {}, false, 8 * 1024 * 1024, NetworkTimeoutMs);
}

void GitRepository::mergeInProgress(const std::function<void(bool)> &done)
{
    run({QStringLiteral("rev-parse"), QStringLiteral("-q"), QStringLiteral("--verify"), QStringLiteral("MERGE_HEAD")},
        [done](const GitResult &result) { done(result.ok()); }, {}, true);
}

void GitRepository::abortMerge(Callback done)
{
    run({QStringLiteral("merge"), QStringLiteral("--abort")}, std::move(done));
}

void GitRepository::stashList(const std::function<void(const GitResult &, const QList<GitStash> &)> &done)
{
    run({QStringLiteral("stash"), QStringLiteral("list"), QString::fromLatin1(stashFormat())},
        [done](const GitResult &result) { done(result, result.ok() ? parseStashList(result.out) : QList<GitStash>()); }, {}, true);
}

void GitRepository::stashPush(const QString &message, bool includeUntracked, Callback done)
{
    QStringList args = {QStringLiteral("stash"), QStringLiteral("push")};
    if (includeUntracked) args << QStringLiteral("--include-untracked");
    if (!message.trimmed().isEmpty()) args << QStringLiteral("-m") << message.trimmed();
    run(args, std::move(done));
}

void GitRepository::stashApply(int index, Callback done)
{
    run({QStringLiteral("stash"), QStringLiteral("apply"), QStringLiteral("stash@{%1}").arg(index)}, std::move(done));
}

void GitRepository::stashPop(int index, Callback done)
{
    run({QStringLiteral("stash"), QStringLiteral("pop"), QStringLiteral("stash@{%1}").arg(index)}, std::move(done));
}

void GitRepository::stashDrop(int index, Callback done)
{
    run({QStringLiteral("stash"), QStringLiteral("drop"), QStringLiteral("stash@{%1}").arg(index)}, std::move(done));
}
