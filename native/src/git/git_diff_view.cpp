#include "git_diff_view.hpp"

#include "git_parsers.hpp"
#include "git_repository.hpp"
#include "../studio_icons.hpp"
#include "../studio_theme.hpp"

#include <QDir>
#include <QFile>
#include <QFontDatabase>
#include <QHBoxLayout>
#include <QLabel>
#include <QPainter>
#include <QPointer>
#include <QScrollBar>
#include <QSettings>
#include <QSplitter>
#include <QStyle>
#include <QTextBlock>
#include <QToolButton>
#include <QVBoxLayout>

namespace {

constexpr int MaxRows = 20000;
constexpr qint64 MaxNewFileBytes = 2 * 1024 * 1024;

QColor tint(const QColor &color, int alpha)
{
    QColor result = color;
    result.setAlpha(alpha);
    return result;
}

QString plural(int count, const QString &one, const QString &many)
{
    return QStringLiteral("%1 %2").arg(count).arg(count == 1 ? one : many);
}

} // namespace

QString GitDiffRequest::key() const
{
    return QStringLiteral("%1|%2|%3").arg(int(source)).arg(path, hash);
}

QString GitDiffRequest::tabTitle() const
{
    return QStringLiteral("Diff · %1").arg(path.section(QLatin1Char('/'), -1));
}

QString GitDiffRequest::badge() const
{
    switch (source) {
    case Source::Staged: return QStringLiteral("Listo para guardar (staged)");
    case Source::Unstaged: return QStringLiteral("Cambios sin preparar");
    case Source::Untracked: return QStringLiteral("Archivo nuevo, sin seguimiento");
    case Source::Head: return QStringLiteral("Cambios desde el último commit");
    case Source::Commit: return QStringLiteral("Commit %1").arg(hash.left(7));
    }
    return {};
}

class GitDiffPane::Gutter final : public QWidget {
public:
    explicit Gutter(GitDiffPane *pane) : QWidget(pane), m_pane(pane) {}
    QSize sizeHint() const override { return {m_pane->gutterWidth(), 0}; }

protected:
    void paintEvent(QPaintEvent *event) override { m_pane->paintGutter(event); }

private:
    GitDiffPane *m_pane;
};

GitDiffPane::GitDiffPane(bool twoNumberColumns, QWidget *parent)
    : QPlainTextEdit(parent), m_twoColumns(twoNumberColumns)
{
    setReadOnly(true);
    setLineWrapMode(QPlainTextEdit::NoWrap);
    setFont(QFontDatabase::systemFont(QFontDatabase::FixedFont));
    setFrameShape(QFrame::NoFrame);
    m_gutter = new Gutter(this);
    connect(this, &QPlainTextEdit::updateRequest, this, [this](const QRect &rect, int dy) {
        if (dy != 0) m_gutter->scroll(0, dy);
        else m_gutter->update(0, rect.y(), m_gutter->width(), rect.height());
    });
    updateGutter();
}

int GitDiffPane::gutterWidth() const
{
    const int digits = fontMetrics().horizontalAdvance(QLatin1Char('9')) * 5;
    return (m_twoColumns ? 2 * digits + 14 : digits + 14) + fontMetrics().horizontalAdvance(QLatin1Char('+'));
}

void GitDiffPane::updateGutter()
{
    setViewportMargins(gutterWidth(), 0, 0, 0);
    const QRect area = contentsRect();
    m_gutter->setGeometry(QRect(area.left(), area.top(), gutterWidth(), area.height()));
}

void GitDiffPane::resizeEvent(QResizeEvent *event)
{
    QPlainTextEdit::resizeEvent(event);
    updateGutter();
}

void GitDiffPane::setLines(const QList<Line> &lines)
{
    m_lines = lines;
    QStringList text;
    text.reserve(lines.size());
    for (const Line &line : lines) text << line.text;
    setPlainText(text.join(QLatin1Char('\n')));
    updateGutter();
    applyHighlights();
}

void GitDiffPane::refreshColors()
{
    applyHighlights();
    m_gutter->update();
}

void GitDiffPane::applyHighlights()
{
    const StudioTheme &theme = ThemeManager::current();
    QList<QTextEdit::ExtraSelection> selections;
    QTextCursor cursor(document());
    cursor.movePosition(QTextCursor::Start);
    for (const Line &line : m_lines) {
        QColor color;
        switch (line.tone) {
        case Tone::Added: color = tint(theme.success, 46); break;
        case Tone::Removed: color = tint(theme.danger, 46); break;
        case Tone::Filler: color = tint(theme.textFaint, 22); break;
        case Tone::Hunk: color = tint(theme.accent, 36); break;
        case Tone::Note: color = tint(theme.warning, 28); break;
        case Tone::None: break;
        }
        if (color.isValid()) {
            QTextEdit::ExtraSelection selection;
            selection.cursor = cursor;
            selection.format.setBackground(color);
            selection.format.setProperty(QTextFormat::FullWidthSelection, true);
            selections.append(selection);
        }
        if (!cursor.movePosition(QTextCursor::NextBlock)) break;
    }
    setExtraSelections(selections);
}

void GitDiffPane::paintGutter(QPaintEvent *event)
{
    const StudioTheme &theme = ThemeManager::current();
    QPainter painter(m_gutter);
    painter.fillRect(event->rect(), theme.editorBackground);
    painter.setFont(font());
    QTextBlock block = firstVisibleBlock();
    int number = block.blockNumber();
    int top = qRound(blockBoundingGeometry(block).translated(contentOffset()).top());
    int bottom = top + qRound(blockBoundingRect(block).height());
    const int digits = fontMetrics().horizontalAdvance(QLatin1Char('9')) * 5;
    while (block.isValid() && top <= event->rect().bottom()) {
        if (block.isVisible() && bottom >= event->rect().top() && number < m_lines.size()) {
            const Line &line = m_lines.at(number);
            painter.setPen(theme.gutterText);
            const int height = fontMetrics().height();
            if (m_twoColumns) {
                if (line.leftNumber > 0) painter.drawText(QRect(0, top, digits, height), Qt::AlignRight, QString::number(line.leftNumber));
                if (line.rightNumber > 0) painter.drawText(QRect(digits + 4, top, digits, height), Qt::AlignRight, QString::number(line.rightNumber));
            } else {
                const int shown = line.rightNumber > 0 ? line.rightNumber : line.leftNumber;
                if (shown > 0) painter.drawText(QRect(0, top, digits, height), Qt::AlignRight, QString::number(shown));
            }
            const QChar sign = line.tone == Tone::Added ? QLatin1Char('+') : (line.tone == Tone::Removed ? QChar(0x2212) : QChar());
            if (!sign.isNull()) {
                painter.setPen(line.tone == Tone::Added ? theme.success : theme.danger);
                painter.drawText(QRect(0, top, m_gutter->width() - 4, height), Qt::AlignRight, QString(sign));
            }
        }
        block = block.next();
        top = bottom;
        bottom = top + qRound(blockBoundingRect(block).height());
        ++number;
    }
}

GitDiffView::GitDiffView(GitRepository *repository, GitDiffRequest request, QWidget *parent)
    : QWidget(parent), m_repository(repository), m_request(std::move(request))
{
    setObjectName(QStringLiteral("gitDiffView"));
    m_sideBySide = QSettings().value(QStringLiteral("git/diffSideBySide"), true).toBool();
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);

    auto *bar = new QWidget(this);
    auto *barLayout = new QHBoxLayout(bar);
    barLayout->setContentsMargins(12, 8, 12, 8);
    m_title = new QLabel(m_request.path, bar);
    QFont bold = m_title->font();
    bold.setBold(true);
    m_title->setFont(bold);
    m_summary = new QLabel(bar);
    barLayout->addWidget(m_title);
    barLayout->addWidget(m_summary);
    barLayout->addStretch(1);
    auto *previous = new QToolButton(bar);
    previous->setIcon(studioIcon(QStringLiteral("go-up"), QStyle::SP_ArrowUp));
    previous->setToolTip(QStringLiteral("Bloque de cambios anterior"));
    auto *next = new QToolButton(bar);
    next->setIcon(studioIcon(QStringLiteral("go-down"), QStyle::SP_ArrowDown));
    next->setToolTip(QStringLiteral("Bloque de cambios siguiente"));
    m_sideButton = new QToolButton(bar);
    m_sideButton->setText(QStringLiteral("Lado a lado"));
    m_sideButton->setIcon(studioIcon(QStringLiteral("view-dual"), QStyle::SP_FileIcon));
    m_inlineButton = new QToolButton(bar);
    m_inlineButton->setText(QStringLiteral("En línea"));
    m_inlineButton->setIcon(studioIcon(QStringLiteral("text-x-generic"), QStyle::SP_FileIcon));
    for (QToolButton *button : {m_sideButton, m_inlineButton}) {
        button->setCheckable(true);
        button->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
        button->setIconSize(QSize(16, 16));
    }
    auto *open = new QToolButton(bar);
    open->setText(QStringLiteral("Abrir archivo"));
    open->setIcon(studioIcon(QStringLiteral("document-open"), QStyle::SP_DirOpenIcon));
    open->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    open->setIconSize(QSize(16, 16));
    barLayout->addWidget(previous);
    barLayout->addWidget(next);
    barLayout->addWidget(m_sideButton);
    barLayout->addWidget(m_inlineButton);
    barLayout->addWidget(open);
    layout->addWidget(bar);

    m_message = new QLabel(this);
    m_message->setAlignment(Qt::AlignCenter);
    m_message->setWordWrap(true);
    m_message->hide();
    layout->addWidget(m_message, 1);

    m_splitter = new QSplitter(Qt::Horizontal, this);
    m_splitter->setChildrenCollapsible(false);
    m_left = new GitDiffPane(false, m_splitter);
    m_right = new GitDiffPane(false, m_splitter);
    m_inline = new GitDiffPane(true, m_splitter);
    layout->addWidget(m_splitter, 1);
    // Both sides have the same number of rows, so keeping their scroll bars equal keeps rows aligned.
    const auto link = [](QScrollBar *a, QScrollBar *b) {
        QObject::connect(a, &QScrollBar::valueChanged, b, &QScrollBar::setValue);
        QObject::connect(b, &QScrollBar::valueChanged, a, &QScrollBar::setValue);
    };
    link(m_left->verticalScrollBar(), m_right->verticalScrollBar());
    link(m_left->horizontalScrollBar(), m_right->horizontalScrollBar());

    // The active mode must be visible: a checked tool button looks like a pressed segment.
    const auto styleModes = [this] {
        const StudioTheme &theme = ThemeManager::current();
        const QString style = QStringLiteral("QToolButton:checked { background-color: %1; color: %2; border-radius: 4px; }")
            .arg(theme.accentSoft.name(), theme.accent.name());
        m_sideButton->setStyleSheet(style);
        m_inlineButton->setStyleSheet(style);
    };
    styleModes();
    connect(ThemeManager::instance(), &ThemeManager::themeChanged, this, styleModes);
    connect(m_sideButton, &QToolButton::clicked, this, [this] { setSideBySide(true); });
    connect(m_inlineButton, &QToolButton::clicked, this, [this] { setSideBySide(false); });
    connect(previous, &QToolButton::clicked, this, [this] { jumpToHunk(-1); });
    connect(next, &QToolButton::clicked, this, [this] { jumpToHunk(1); });
    connect(open, &QToolButton::clicked, this, [this] { emit openFileRequested(m_request.path); });
    connect(ThemeManager::instance(), &ThemeManager::themeChanged, this, [this] {
        for (GitDiffPane *pane : {m_left, m_right, m_inline}) pane->refreshColors();
    });
    showMessage(QStringLiteral("Cargando cambios…"));
    reload();
}

void GitDiffView::reload()
{
    // The tab can be closed while git is still working: drop results that arrive for a deleted view.
    QPointer<GitDiffView> self(this);
    const auto done = [self](const GitResult &result, const GitDiff &diff) {
        if (self.isNull()) return;
        if (!result.ok()) return self->showMessage(result.message());
        self->setDiff(diff);
    };
    switch (m_request.source) {
    case GitDiffRequest::Source::Staged: m_repository->fileDiff(m_request.path, true, done); break;
    case GitDiffRequest::Source::Unstaged: m_repository->fileDiff(m_request.path, false, done); break;
    case GitDiffRequest::Source::Head: m_repository->headDiff(m_request.path, done); break;
    case GitDiffRequest::Source::Commit: m_repository->commitDiff(m_request.hash, m_request.path, done); break;
    case GitDiffRequest::Source::Untracked: {
        // git has no diff for a file it does not track: show it as all additions.
        QFile file(QDir(m_repository->root()).filePath(m_request.path));
        if (!file.open(QIODevice::ReadOnly)) return showMessage(QStringLiteral("No pude leer el archivo."));
        const QByteArray bytes = file.read(MaxNewFileBytes + 1);
        if (bytes.contains('\0')) {
            GitDiff binary;
            binary.binary = true;
            return setDiff(binary);
        }
        if (bytes.size() > MaxNewFileBytes) return showMessage(QStringLiteral("El archivo es demasiado grande para mostrarlo aquí."));
        setDiff(diffForNewFile(QString::fromUtf8(bytes)));
        break;
    }
    }
}

void GitDiffView::setDiff(const GitDiff &diff)
{
    m_diff = diff;
    m_currentHunk = -1;
    render();
}

void GitDiffView::setSideBySide(bool value)
{
    m_sideBySide = value;
    QSettings().setValue(QStringLiteral("git/diffSideBySide"), value);
    render();
}

void GitDiffView::showMessage(const QString &text)
{
    m_message->setText(text);
    m_message->show();
    m_splitter->hide();
    m_summary->clear();
}

void GitDiffView::render()
{
    m_sideButton->setChecked(m_sideBySide);
    m_inlineButton->setChecked(!m_sideBySide);
    if (m_diff.binary) return showMessage(QStringLiteral("Archivo binario: no hay vista previa de los cambios."));
    if (m_diff.rows.isEmpty()) return showMessage(QStringLiteral("Sin cambios en este archivo."));
    m_message->hide();
    m_splitter->show();
    m_summary->setText(QStringLiteral("%1 · %2 · %3").arg(m_request.badge(), plural(m_diff.additions, QStringLiteral("línea añadida"), QStringLiteral("líneas añadidas")),
        plural(m_diff.deletions, QStringLiteral("eliminada"), QStringLiteral("eliminadas"))));

    using Tone = GitDiffPane::Tone;
    QList<GitDiffPane::Line> left;
    QList<GitDiffPane::Line> right;
    QList<GitDiffPane::Line> inlineLines;
    m_hunkBlocks.clear();
    const int rows = qMin<int>(m_diff.rows.size(), MaxRows);
    for (int i = 0; i < rows; ++i) {
        const GitDiffRow &row = m_diff.rows.at(i);
        switch (row.kind) {
        case GitDiffRow::Kind::Hunk:
            m_hunkBlocks << (m_sideBySide ? left.size() : inlineLines.size());
            left.append({row.left, 0, 0, Tone::Hunk});
            right.append({QString(), 0, 0, Tone::Hunk});
            inlineLines.append({row.left, 0, 0, Tone::Hunk});
            break;
        case GitDiffRow::Kind::Note:
            left.append({row.left, 0, 0, Tone::Note});
            right.append({row.right, 0, 0, Tone::Note});
            inlineLines.append({row.left, 0, 0, Tone::Note});
            break;
        case GitDiffRow::Kind::Context:
            left.append({row.left, row.leftNumber, 0, Tone::None});
            right.append({row.right, 0, row.rightNumber, Tone::None});
            inlineLines.append({row.left, row.leftNumber, row.rightNumber, Tone::None});
            break;
        case GitDiffRow::Kind::Removed:
            left.append({row.left, row.leftNumber, 0, Tone::Removed});
            right.append({QString(), 0, 0, Tone::Filler});
            inlineLines.append({row.left, row.leftNumber, 0, Tone::Removed});
            break;
        case GitDiffRow::Kind::Added:
            left.append({QString(), 0, 0, Tone::Filler});
            right.append({row.right, 0, row.rightNumber, Tone::Added});
            inlineLines.append({row.right, 0, row.rightNumber, Tone::Added});
            break;
        case GitDiffRow::Kind::Changed:
            left.append({row.left, row.leftNumber, 0, Tone::Removed});
            right.append({row.right, 0, row.rightNumber, Tone::Added});
            inlineLines.append({row.left, row.leftNumber, 0, Tone::Removed});
            inlineLines.append({row.right, 0, row.rightNumber, Tone::Added});
            break;
        }
    }
    if (m_diff.rows.size() > MaxRows) {
        const QString note = QStringLiteral("… diff recortado a %1 filas").arg(MaxRows);
        left.append({note, 0, 0, Tone::Note});
        right.append({note, 0, 0, Tone::Note});
        inlineLines.append({note, 0, 0, Tone::Note});
    }
    // The inline view numbers lines separately: recompute where hunks start there.
    if (!m_sideBySide) {
        m_hunkBlocks.clear();
        for (int i = 0; i < inlineLines.size(); ++i) if (inlineLines.at(i).tone == Tone::Hunk) m_hunkBlocks << i;
    }
    m_left->setLines(left);
    m_right->setLines(right);
    m_inline->setLines(inlineLines);
    m_left->setVisible(m_sideBySide);
    m_right->setVisible(m_sideBySide);
    m_inline->setVisible(!m_sideBySide);
    // Side by side: the left pane carries the numbers of the old file, the right pane those of the new.
    if (m_hunkBlocks.size() > 0 && m_currentHunk < 0) jumpToHunk(1);
}

void GitDiffView::jumpToHunk(int step)
{
    if (m_hunkBlocks.isEmpty()) return;
    m_currentHunk = qBound(0, m_currentHunk + step, int(m_hunkBlocks.size()) - 1);
    GitDiffPane *pane = m_sideBySide ? m_left : m_inline;
    QTextCursor cursor(pane->document()->findBlockByNumber(m_hunkBlocks.at(m_currentHunk)));
    pane->setTextCursor(cursor);
    pane->centerCursor();
}

QString GitDiffView::statusText() const
{
    return m_message->isVisibleTo(this) ? m_message->text() : m_summary->text();
}

int GitDiffView::paneCount() const
{
    return (m_left->isVisibleTo(this) ? 1 : 0) + (m_right->isVisibleTo(this) ? 1 : 0) + (m_inline->isVisibleTo(this) ? 1 : 0);
}

int GitDiffView::visibleLineCount() const
{
    return m_sideBySide ? m_left->blockCount() : m_inline->blockCount();
}
