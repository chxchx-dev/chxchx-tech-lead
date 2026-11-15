#include "git_commit_plan.hpp"

#include "git_assistant_prompts.hpp"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSet>

namespace {

constexpr int MaxGroups = 8;

QJsonObject planObject(const QString &raw)
{
    // The agent may wrap the JSON in a fence or add a sentence around it: take the outermost object.
    const qsizetype start = raw.indexOf(QLatin1Char('{'));
    const qsizetype end = raw.lastIndexOf(QLatin1Char('}'));
    if (start < 0 || end <= start) return {};
    return QJsonDocument::fromJson(raw.mid(start, end - start + 1).toUtf8()).object();
}

} // namespace

QList<GitCommitGroup> parseCommitPlan(const QString &raw, const QStringList &pendingFiles)
{
    const QJsonArray commits = planObject(raw).value(QStringLiteral("commits")).toArray();
    if (commits.isEmpty()) return {};
    const QSet<QString> pending(pendingFiles.begin(), pendingFiles.end());
    QSet<QString> used;
    QList<GitCommitGroup> plan;
    for (const QJsonValue &value : commits) {
        if (plan.size() >= MaxGroups) break;
        const QJsonObject item = value.toObject();
        GitCommitGroup group;
        group.message = cleanCommitMessage(item.value(QStringLiteral("message")).toString());
        for (const QJsonValue &file : item.value(QStringLiteral("files")).toArray()) {
            const QString path = file.toString();
            if (pending.contains(path) && !used.contains(path)) {
                used.insert(path);
                group.files << path;
            }
        }
        if (!group.message.isEmpty() && !group.files.isEmpty()) plan.append(group);
    }
    if (plan.isEmpty()) return {};
    QStringList left;
    for (const QString &path : pendingFiles) if (pending.contains(path) && !used.contains(path)) left << path;
    if (!left.isEmpty()) plan.append({QStringLiteral("Otros cambios"), left});
    return plan;
}

QList<GitCommitGroup> reconcileCommitPlan(const QList<GitCommitGroup> &plan, const QStringList &pendingFiles)
{
    const QSet<QString> pending(pendingFiles.begin(), pendingFiles.end());
    QList<GitCommitGroup> result;
    for (GitCommitGroup group : plan) {
        QStringList kept;
        for (const QString &path : group.files) if (pending.contains(path)) kept << path;
        group.files = kept;
        if (!group.files.isEmpty()) result.append(group);
    }
    return result;
}
