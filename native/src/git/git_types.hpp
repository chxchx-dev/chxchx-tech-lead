#pragma once

#include <QDateTime>
#include <QList>
#include <QString>
#include <QStringList>
#include <QVector>

// Plain data produced by parsing `git` output; no Qt widgets and no processes here,
// so every shape can be tested from literal text.

struct GitEntry {
    enum class Kind { Ordinary, Renamed, Unmerged, Untracked };
    Kind kind = Kind::Ordinary;
    QString path;
    QString origPath; // renames only
    QChar index = QLatin1Char('.'); // X of the porcelain status: change already prepared for commit
    QChar work = QLatin1Char('.');  // Y: change in the working tree

    bool conflicted() const { return kind == Kind::Unmerged; }
    bool untracked() const { return kind == Kind::Untracked; }
    bool staged() const { return (kind == Kind::Ordinary || kind == Kind::Renamed) && index != QLatin1Char('.'); }
    bool unstaged() const { return (kind == Kind::Ordinary || kind == Kind::Renamed) && work != QLatin1Char('.'); }
};

struct GitStatus {
    QString head; // branch name, or the short oid when detached
    QString oid;
    bool detached = false;
    bool initial = false; // no commits yet
    QString upstream;
    int ahead = 0;
    int behind = 0;
    QList<GitEntry> entries;

    QList<GitEntry> conflicts() const;
    QList<GitEntry> stagedEntries() const;
    // Edited tracked files plus files git does not track yet.
    QList<GitEntry> changedEntries() const;
};

struct GitBranch {
    QString name; // "main" or "origin/main"
    bool current = false;
    bool remote = false;
    QString upstream;
    QString track; // "ahead 2, behind 1"
    QString hash;
    QString subject;
};

// How one history row is drawn. Lanes are columns; the node sits on `lane`.
struct GitGraphRow {
    int lane = 0;
    bool incoming = false;        // a line arrives at the node from above
    QVector<int> through;         // lanes whose line passes straight through the row
    QVector<int> mergeIn;         // other lanes that curve into the node from above
    QVector<int> parentLanes;     // lane each parent continues on, in parent order
    int laneCount = 1;
};

struct GitCommit {
    QString hash;
    QStringList parents;
    QString author;
    QDateTime time;
    QStringList refs; // "HEAD -> main", "origin/main", "tag: v1"
    QString subject;
    GitGraphRow graph;

    QString shortHash() const { return hash.left(7); }
};

struct GitStash {
    int index = 0;      // stash@{index}
    QString message;    // "On main: arreglo a medias"
    QDateTime time;
};

struct GitFileChange {
    QString status; // M A D R C T
    QString path;
    QString origPath;
};

struct GitDiffRow {
    enum class Kind { Context, Added, Removed, Changed, Hunk, Note };
    Kind kind = Kind::Context;
    int leftNumber = 0;  // 0 = none
    int rightNumber = 0;
    QString left;
    QString right;
};

struct GitDiff {
    bool binary = false;
    int additions = 0;
    int deletions = 0;
    QList<GitDiffRow> rows;
    QList<int> hunkRows; // indexes into `rows`
};
