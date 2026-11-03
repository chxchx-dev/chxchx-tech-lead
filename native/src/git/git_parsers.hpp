#pragma once

#include "git_types.hpp"

// `git status --porcelain=v2 --branch -z --untracked-files=all`
GitStatus parseStatusV2(const QByteArray &output);
// `git for-each-ref --format=<branchFormat()> refs/heads refs/remotes`
QByteArray branchFormat();
QList<GitBranch> parseBranches(const QByteArray &output);
// `git log -z --format=<logFormat()>`
QByteArray logFormat();
QList<GitCommit> parseLog(const QByteArray &output);
// Assigns graph lanes in display order. `lanes` carries the open lines between pages of history.
void assignGraph(QList<GitCommit> &commits, QVector<QString> &lanes);
// `git diff` / `git show` unified output for one file.
GitDiff parseUnifiedDiff(const QByteArray &output);
// A file git does not track yet: every line is an addition.
GitDiff diffForNewFile(const QString &text);
// `git stash list --format=<stashFormat()>`
QByteArray stashFormat();
QList<GitStash> parseStashList(const QByteArray &output);
// `git show --format= --name-status -z`
QList<GitFileChange> parseNameStatus(const QByteArray &output);
// Human label for a porcelain letter: "Modificado", "Nuevo"…
QString changeLabel(QChar letter, bool untracked = false);
