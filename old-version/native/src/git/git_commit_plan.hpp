#pragma once

#include <QList>
#include <QStringList>

// One commit the assistant proposes: what it says and which files belong in it.
struct GitCommitGroup {
    QString message;
    QStringList files;
};

// Reads the assistant's answer as a plan. Only files that are really pending are kept, each in one group;
// whatever the assistant left out goes to a last group so nothing is lost. An answer that is not a plan
// (a plain message) gives an empty list, and the caller treats it as a single message.
QList<GitCommitGroup> parseCommitPlan(const QString &raw, const QStringList &pendingFiles);
// The groups that still have pending files, after some were committed or edited.
QList<GitCommitGroup> reconcileCommitPlan(const QList<GitCommitGroup> &plan, const QStringList &pendingFiles);
