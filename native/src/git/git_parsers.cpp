#include "git_parsers.hpp"

#include <QRegularExpression>

namespace {

constexpr char FieldSeparator = '\x1f';

QList<QByteArray> splitNul(const QByteArray &data)
{
    QList<QByteArray> parts = data.split('\0');
    if (!parts.isEmpty() && parts.last().isEmpty()) parts.removeLast();
    return parts;
}

QString fromGit(const QByteArray &bytes)
{
    return QString::fromUtf8(bytes);
}

} // namespace

QList<GitEntry> GitStatus::conflicts() const
{
    QList<GitEntry> result;
    for (const GitEntry &entry : entries) if (entry.conflicted()) result.append(entry);
    return result;
}

QList<GitEntry> GitStatus::stagedEntries() const
{
    QList<GitEntry> result;
    for (const GitEntry &entry : entries) if (entry.staged()) result.append(entry);
    return result;
}

QList<GitEntry> GitStatus::changedEntries() const
{
    QList<GitEntry> result;
    for (const GitEntry &entry : entries) if (entry.unstaged() || entry.untracked()) result.append(entry);
    return result;
}

GitStatus parseStatusV2(const QByteArray &output)
{
    GitStatus status;
    const QList<QByteArray> records = splitNul(output);
    for (int i = 0; i < records.size(); ++i) {
        const QString line = fromGit(records.at(i));
        if (line.startsWith(QLatin1String("# "))) {
            const QStringList parts = line.mid(2).split(QLatin1Char(' '));
            const QString key = parts.value(0);
            if (key == QLatin1String("branch.oid")) {
                status.oid = parts.value(1);
                status.initial = status.oid == QLatin1String("(initial)");
            } else if (key == QLatin1String("branch.head")) {
                status.detached = parts.value(1) == QLatin1String("(detached)");
                status.head = status.detached ? QString() : parts.value(1);
            } else if (key == QLatin1String("branch.upstream")) {
                status.upstream = parts.value(1);
            } else if (key == QLatin1String("branch.ab")) {
                status.ahead = parts.value(1).mid(1).toInt();
                status.behind = parts.value(2).mid(1).toInt();
            }
            continue;
        }
        GitEntry entry;
        if (line.startsWith(QLatin1String("? "))) {
            entry.kind = GitEntry::Kind::Untracked;
            entry.path = line.mid(2);
        } else if (line.startsWith(QLatin1String("1 ")) || line.startsWith(QLatin1String("2 "))
                   || line.startsWith(QLatin1String("u "))) {
            const QChar type = line.at(0);
            // The path is the last field and may contain spaces: split with a limit.
            const int fields = type == QLatin1Char('1') ? 9 : (type == QLatin1Char('2') ? 10 : 11);
            const QStringList parts = line.split(QLatin1Char(' '), Qt::KeepEmptyParts).mid(0, fields - 1);
            int offset = 0;
            for (const QString &part : parts) offset += part.size() + 1;
            const QString xy = parts.value(1);
            entry.index = xy.size() > 0 ? xy.at(0) : QLatin1Char('.');
            entry.work = xy.size() > 1 ? xy.at(1) : QLatin1Char('.');
            entry.path = line.mid(offset);
            if (type == QLatin1Char('u')) {
                entry.kind = GitEntry::Kind::Unmerged;
            } else if (type == QLatin1Char('2')) {
                entry.kind = GitEntry::Kind::Renamed;
                entry.origPath = fromGit(records.value(++i)); // the previous path follows as its own record
            }
        } else {
            continue; // ignored files ("! path") and anything unknown
        }
        status.entries.append(entry);
    }
    return status;
}

QByteArray branchFormat()
{
    return "--format=%(HEAD)%1f%(refname)%1f%(upstream:short)%1f%(upstream:track,nobracket)%1f%(objectname:short)%1f%(contents:subject)";
}

QList<GitBranch> parseBranches(const QByteArray &output)
{
    QList<GitBranch> branches;
    for (const QByteArray &raw : output.split('\n')) {
        const QStringList parts = fromGit(raw).split(QLatin1Char(FieldSeparator));
        if (parts.size() < 6 || parts.at(1).isEmpty()) continue;
        GitBranch branch;
        branch.current = parts.at(0) == QLatin1String("*");
        const QString ref = parts.at(1);
        branch.remote = ref.startsWith(QLatin1String("refs/remotes/"));
        branch.name = ref.mid(branch.remote ? 13 : 11);
        if (branch.remote && branch.name.endsWith(QLatin1String("/HEAD"))) continue;
        branch.upstream = parts.at(2);
        branch.track = parts.at(3);
        branch.hash = parts.at(4);
        branch.subject = parts.mid(5).join(QString(QLatin1Char(FieldSeparator)));
        branches.append(branch);
    }
    return branches;
}

QByteArray logFormat()
{
    return "--format=%H%x1f%P%x1f%an%x1f%at%x1f%D%x1f%s";
}

QList<GitCommit> parseLog(const QByteArray &output)
{
    QList<GitCommit> commits;
    for (const QByteArray &record : splitNul(output)) {
        const QStringList parts = fromGit(record).trimmed().split(QLatin1Char(FieldSeparator));
        if (parts.size() < 6 || parts.at(0).isEmpty()) continue;
        GitCommit commit;
        commit.hash = parts.at(0);
        commit.parents = parts.at(1).split(QLatin1Char(' '), Qt::SkipEmptyParts);
        commit.author = parts.at(2);
        commit.time = QDateTime::fromSecsSinceEpoch(parts.at(3).toLongLong());
        for (const QString &ref : parts.at(4).split(QLatin1String(", "), Qt::SkipEmptyParts)) commit.refs << ref.trimmed();
        commit.subject = parts.mid(5).join(QString(QLatin1Char(FieldSeparator)));
        commits.append(commit);
    }
    return commits;
}

void assignGraph(QList<GitCommit> &commits, QVector<QString> &lanes)
{
    const auto freeLane = [&lanes] {
        for (int i = 0; i < lanes.size(); ++i) if (lanes.at(i).isEmpty()) return i;
        lanes.append(QString());
        return int(lanes.size()) - 1;
    };
    for (GitCommit &commit : commits) {
        GitGraphRow row;
        QVector<int> waiting;
        for (int i = 0; i < lanes.size(); ++i) if (lanes.at(i) == commit.hash) waiting.append(i);
        row.incoming = !waiting.isEmpty();
        row.lane = waiting.isEmpty() ? freeLane() : waiting.first();
        for (int i = 0; i < lanes.size(); ++i) {
            if (!lanes.at(i).isEmpty() && lanes.at(i) != commit.hash) row.through.append(i);
        }
        for (int i = 1; i < waiting.size(); ++i) {
            row.mergeIn.append(waiting.at(i));
            lanes[waiting.at(i)].clear();
        }
        if (commit.parents.isEmpty()) {
            lanes[row.lane].clear();
        } else {
            lanes[row.lane] = commit.parents.first();
            row.parentLanes.append(row.lane);
            for (int p = 1; p < commit.parents.size(); ++p) {
                const QString &parent = commit.parents.at(p);
                int target = lanes.indexOf(parent);
                if (target < 0) {
                    target = freeLane();
                    lanes[target] = parent;
                }
                row.parentLanes.append(target);
            }
        }
        int widest = row.lane;
        for (int lane : row.through) widest = qMax(widest, lane);
        for (int lane : row.mergeIn) widest = qMax(widest, lane);
        for (int lane : row.parentLanes) widest = qMax(widest, lane);
        row.laneCount = widest + 1;
        commit.graph = row;
        while (!lanes.isEmpty() && lanes.last().isEmpty()) lanes.removeLast();
    }
}

GitDiff parseUnifiedDiff(const QByteArray &output)
{
    GitDiff diff;
    static const QRegularExpression hunkHeader(QStringLiteral(R"(^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@)"));
    const QStringList lines = fromGit(output).split(QLatin1Char('\n'));
    int left = 0;
    int right = 0;
    bool inHunk = false;
    QStringList removed;
    QStringList added;
    int removedStart = 0;
    int addedStart = 0;

    const auto flush = [&] {
        const int paired = qMin(removed.size(), added.size());
        for (int i = 0; i < paired; ++i) {
            diff.rows.append({GitDiffRow::Kind::Changed, removedStart + i, addedStart + i, removed.at(i), added.at(i)});
        }
        for (int i = paired; i < removed.size(); ++i)
            diff.rows.append({GitDiffRow::Kind::Removed, removedStart + i, 0, removed.at(i), QString()});
        for (int i = paired; i < added.size(); ++i)
            diff.rows.append({GitDiffRow::Kind::Added, 0, addedStart + i, QString(), added.at(i)});
        removed.clear();
        added.clear();
    };

    for (const QString &line : lines) {
        if (line.startsWith(QLatin1String("Binary files ")) || line.startsWith(QLatin1String("GIT binary patch"))) {
            diff.binary = true;
            continue;
        }
        const QRegularExpressionMatch match = hunkHeader.match(line);
        if (match.hasMatch()) {
            flush();
            inHunk = true;
            left = match.captured(1).toInt();
            right = match.captured(2).toInt();
            diff.hunkRows.append(diff.rows.size());
            diff.rows.append({GitDiffRow::Kind::Hunk, 0, 0, line, line});
            continue;
        }
        if (!inHunk) continue;
        if (line.startsWith(QLatin1Char('-'))) {
            if (removed.isEmpty()) removedStart = left;
            removed << line.mid(1);
            ++left;
            ++diff.deletions;
        } else if (line.startsWith(QLatin1Char('+'))) {
            if (added.isEmpty()) addedStart = right;
            added << line.mid(1);
            ++right;
            ++diff.additions;
        } else if (line.startsWith(QLatin1Char('\\'))) {
            flush();
            diff.rows.append({GitDiffRow::Kind::Note, 0, 0, line.mid(2), line.mid(2)});
        } else if (line.startsWith(QLatin1Char(' '))) {
            flush();
            diff.rows.append({GitDiffRow::Kind::Context, left++, right++, line.mid(1), line.mid(1)});
        }
    }
    flush();
    return diff;
}

GitDiff diffForNewFile(const QString &text)
{
    GitDiff diff;
    QStringList lines = text.split(QLatin1Char('\n'));
    if (!lines.isEmpty() && lines.last().isEmpty()) lines.removeLast();
    diff.hunkRows.append(0);
    diff.rows.append({GitDiffRow::Kind::Hunk, 0, 0, QStringLiteral("Archivo nuevo"), QStringLiteral("Archivo nuevo")});
    for (int i = 0; i < lines.size(); ++i) {
        diff.rows.append({GitDiffRow::Kind::Added, 0, i + 1, QString(), lines.at(i)});
    }
    diff.additions = lines.size();
    return diff;
}

QList<GitFileChange> parseNameStatus(const QByteArray &output)
{
    QList<GitFileChange> changes;
    const QList<QByteArray> parts = splitNul(output);
    for (int i = 0; i < parts.size(); ++i) {
        const QString status = fromGit(parts.at(i)).trimmed();
        if (status.isEmpty()) continue;
        GitFileChange change;
        change.status = status.left(1);
        if (change.status == QLatin1String("R") || change.status == QLatin1String("C")) {
            change.origPath = fromGit(parts.value(++i));
        }
        change.path = fromGit(parts.value(++i));
        changes.append(change);
    }
    return changes;
}

QString changeLabel(QChar letter, bool untracked)
{
    if (untracked) return QStringLiteral("Nuevo (sin seguimiento)");
    switch (letter.toLatin1()) {
    case 'M': return QStringLiteral("Modificado");
    case 'A': return QStringLiteral("Nuevo");
    case 'D': return QStringLiteral("Eliminado");
    case 'R': return QStringLiteral("Renombrado");
    case 'C': return QStringLiteral("Copiado");
    case 'T': return QStringLiteral("Tipo cambiado");
    case 'U': return QStringLiteral("Conflicto");
    default: return QStringLiteral("Cambiado");
    }
}

QByteArray stashFormat()
{
    return "--format=%gd%x1f%at%x1f%gs";
}

QList<GitStash> parseStashList(const QByteArray &output)
{
    static const QRegularExpression reference(QStringLiteral(R"(^stash@\{(\d+)\}$)"));
    QList<GitStash> stashes;
    for (const QByteArray &raw : output.split('\n')) {
        const QStringList parts = fromGit(raw).split(QLatin1Char(FieldSeparator));
        if (parts.size() < 3) continue;
        const QRegularExpressionMatch match = reference.match(parts.at(0));
        if (!match.hasMatch()) continue;
        GitStash stash;
        stash.index = match.captured(1).toInt();
        stash.time = QDateTime::fromSecsSinceEpoch(parts.at(1).toLongLong());
        stash.message = parts.mid(2).join(QString(QLatin1Char(FieldSeparator)));
        stashes.append(stash);
    }
    return stashes;
}
