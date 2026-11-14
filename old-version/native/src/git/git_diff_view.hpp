#pragma once

#include "git_types.hpp"

#include <QPlainTextEdit>
#include <QWidget>

class GitRepository;
class QLabel;
class QSplitter;
class QToolButton;

// What a diff tab shows.
struct GitDiffRequest {
    enum class Source { Unstaged, Staged, Untracked, Head, Commit };
    Source source = Source::Unstaged;
    QString path; // relative to the repository root
    QString hash; // for Source::Commit

    QString key() const;
    QString tabTitle() const;
    QString badge() const;
};

// One side of the diff: text with a line-number gutter and a colour per row.
class GitDiffPane final : public QPlainTextEdit {
    Q_OBJECT
public:
    enum class Tone { None, Added, Removed, Filler, Hunk, Note };
    struct Line {
        QString text;
        int leftNumber = 0;
        int rightNumber = 0;
        Tone tone = Tone::None;
    };
    explicit GitDiffPane(bool twoNumberColumns, QWidget *parent = nullptr);
    void setLines(const QList<Line> &lines);
    int gutterWidth() const;
    void paintGutter(QPaintEvent *event);
    void refreshColors();

protected:
    void resizeEvent(QResizeEvent *event) override;

private:
    class Gutter;
    void updateGutter();
    void applyHighlights();

    bool m_twoColumns;
    QList<Line> m_lines;
    Gutter *m_gutter = nullptr;
};

// A readable diff: side by side or inline, with change counts and jumps between blocks.
class GitDiffView final : public QWidget {
    Q_OBJECT
public:
    GitDiffView(GitRepository *repository, GitDiffRequest request, QWidget *parent = nullptr);
    const GitDiffRequest &request() const { return m_request; }
    void reload();
    void setDiff(const GitDiff &diff);
    bool sideBySide() const { return m_sideBySide; }
    void setSideBySide(bool value);
    // For tests and accessibility.
    QString statusText() const;
    int paneCount() const;
    int visibleLineCount() const;
    void jumpToHunk(int step);

signals:
    void openFileRequested(const QString &relativePath);

private:
    void render();
    void showMessage(const QString &text);

    GitRepository *m_repository;
    GitDiffRequest m_request;
    GitDiff m_diff;
    bool m_sideBySide = true;
    int m_currentHunk = -1;
    QLabel *m_title = nullptr;
    QLabel *m_summary = nullptr;
    QLabel *m_message = nullptr;
    QToolButton *m_sideButton = nullptr;
    QToolButton *m_inlineButton = nullptr;
    QSplitter *m_splitter = nullptr;
    GitDiffPane *m_left = nullptr;
    GitDiffPane *m_right = nullptr;
    GitDiffPane *m_inline = nullptr;
    QList<int> m_hunkBlocks;
};
