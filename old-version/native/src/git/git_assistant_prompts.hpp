#pragma once

#include "git_types.hpp"

// What the Git assistant can be asked about.
enum class GitAdviceKind { WhatNext, Conflicts, Diverged, PushRejected, Credentials, Failure };

// Everything the assistant is told. It is built from the repository, never from the user's whole disk.
struct GitAssistContext {
    GitStatus status;
    QStringList recentSubjects;   // for the style of the message
    QString diff;                 // prepared changes, or all of them when nothing is prepared
    QStringList conflictFiles;
    QString conflictExcerpts;     // the conflicting blocks of those files
    bool mergeInProgress = false;
    int stashCount = 0;
    QString lastError;            // what Git said when something failed
};

// Files that may hold secrets are never sent.
bool isSensitivePath(const QString &path);
// Drops the diff sections of sensitive files and masks values that look like secrets.
// `omittedFiles` receives the paths that were left out.
QString redactDiff(const QString &diff, QStringList *omittedFiles = nullptr);
QString redactSecrets(const QString &text);
// Keeps the start of `text` when it is longer than `maxChars` and says so.
QString truncateForPrompt(const QString &text, int maxChars, bool *truncated = nullptr);
// The `<<<<<<<`..`>>>>>>>` blocks of a file with conflicts, a few lines each.
QString extractConflictHunks(const QString &fileText, int maxHunks = 3, int maxLines = 40);

// A file git does not track yet, written as the diff it would be, so it is redacted like any other.
QString newFileDiff(const QString &path, const QString &text, int maxChars = 4000);
QString buildCommitPrompt(const GitAssistContext &context);
// The files a commit suggestion is about: the prepared ones, or all pending when nothing is prepared (secrets excluded).
QStringList commitCandidateFiles(const GitStatus &status);
QString buildAdvicePrompt(GitAdviceKind kind, const GitAssistContext &context);
// The agent's answer reduced to a commit message: no fences, quotes or "Mensaje:" lead-in.
QString cleanCommitMessage(const QString &raw);
QString adviceTitle(GitAdviceKind kind);
