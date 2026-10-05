#include "vt_terminal_widget.hpp"

#include <vterm.h>

#include <QFontDatabase>
#include <QApplication>
#include <QClipboard>
#include <QEvent>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QPainter>
#include <QResizeEvent>
#include <QWheelEvent>

#include <algorithm>
#include <cstdlib>

struct VtTerminalWidget::State {
    VTerm *terminal = nullptr;
    VTermScreen *screen = nullptr;
    VTermPos cursor{0, 0};
    bool cursorVisible = true;
    int columns = 80;
    int rows = 24;
    qreal cellWidth = 8;
    qreal cellHeight = 18;
    qreal ascent = 14;
};

VtTerminalWidget::VtTerminalWidget(QWidget *parent)
    : QWidget(parent), m_state(std::make_unique<State>())
{
    setFocusPolicy(Qt::StrongFocus);
    setMouseTracking(true);
    setAttribute(Qt::WA_OpaquePaintEvent);
    QFont font = QFontDatabase::systemFont(QFontDatabase::FixedFont);
    font.setPointSize(12);
    setFont(font);
    const QFontMetricsF metrics(font);
    m_state->cellWidth = std::max<qreal>(1, metrics.horizontalAdvance(QLatin1Char('M')));
    m_state->cellHeight = std::max<qreal>(1, metrics.height());
    m_state->ascent = metrics.ascent();
    m_state->terminal = vterm_new(m_state->rows, m_state->columns);
    vterm_set_utf8(m_state->terminal, 1);
    m_state->screen = vterm_obtain_screen(m_state->terminal);
    static const VTermScreenCallbacks callbacks{
        &VtTerminalWidget::damage, nullptr, &VtTerminalWidget::cursorMoved,
        nullptr, nullptr, nullptr, nullptr, nullptr, nullptr};
    vterm_screen_set_callbacks(m_state->screen, &callbacks, this);
    vterm_screen_set_damage_merge(m_state->screen, VTERM_DAMAGE_ROW);
    vterm_screen_enable_altscreen(m_state->screen, 1);
    vterm_screen_reset(m_state->screen, 1);
}

VtTerminalWidget::~VtTerminalWidget()
{
    if (m_state->terminal) vterm_free(m_state->terminal);
}

bool VtTerminalWidget::event(QEvent *event)
{
    if (event->type() == QEvent::ShortcutOverride) {
        event->accept();
        return true;
    }
    return QWidget::event(event);
}

int VtTerminalWidget::damage(VTermRect rect, void *user)
{
    auto *widget = static_cast<VtTerminalWidget *>(user);
    const qreal cellWidth = widget->m_state->cellWidth;
    const qreal cellHeight = widget->m_state->cellHeight;
    widget->update(QRectF(rect.start_col * cellWidth, rect.start_row * cellHeight,
        (rect.end_col - rect.start_col) * cellWidth,
        (rect.end_row - rect.start_row) * cellHeight).toAlignedRect());
    return 1;
}

int VtTerminalWidget::cursorMoved(VTermPos pos, VTermPos oldPos, int visible, void *user)
{
    auto *widget = static_cast<VtTerminalWidget *>(user);
    widget->m_state->cursor = pos;
    widget->m_state->cursorVisible = visible != 0;
    const qreal width = widget->m_state->cellWidth;
    const qreal height = widget->m_state->cellHeight;
    widget->update(QRectF(oldPos.col * width, oldPos.row * height, width, height).toAlignedRect());
    widget->update(QRectF(pos.col * width, pos.row * height, width, height).toAlignedRect());
    return 1;
}

void VtTerminalWidget::feed(const QByteArray &bytes)
{
    if (!bytes.isEmpty()) vterm_input_write(m_state->terminal, bytes.constData(), static_cast<size_t>(bytes.size()));
    vterm_screen_flush_damage(m_state->screen);
    flushInput();
}

void VtTerminalWidget::flushInput()
{
    QByteArray bytes(static_cast<qsizetype>(vterm_output_get_buffer_current(m_state->terminal)), '\0');
    if (!bytes.isEmpty()) {
        const size_t count = vterm_output_read(m_state->terminal, bytes.data(), static_cast<size_t>(bytes.size()));
        bytes.resize(static_cast<qsizetype>(count));
        emit inputReady(bytes);
    }
}

void VtTerminalWidget::paintEvent(QPaintEvent *event)
{
    QPainter painter(this);
    painter.setClipRegion(event->region());
    painter.fillRect(event->rect(), QColor(18, 20, 25));
    painter.setFont(font());
    const QColor defaultForeground(220, 224, 232);
    const QColor defaultBackground(18, 20, 25);
    const QRect dirty = event->rect();
    const int firstRow = std::clamp(static_cast<int>(dirty.top() / m_state->cellHeight), 0, m_state->rows - 1);
    const int lastRow = std::clamp(static_cast<int>(dirty.bottom() / m_state->cellHeight), 0, m_state->rows - 1);
    const int firstColumn = std::clamp(static_cast<int>(dirty.left() / m_state->cellWidth), 0, m_state->columns - 1);
    const int lastColumn = std::clamp(static_cast<int>(dirty.right() / m_state->cellWidth), 0, m_state->columns - 1);
    for (int row = firstRow; row <= lastRow; ++row) {
        for (int column = firstColumn; column <= lastColumn; ++column) {
            VTermScreenCell cell{};
            if (!vterm_screen_get_cell(m_state->screen, VTermPos{row, column}, &cell)) continue;
            VTermColor foreground = cell.fg;
            VTermColor background = cell.bg;
            vterm_screen_convert_color_to_rgb(m_state->screen, &foreground);
            vterm_screen_convert_color_to_rgb(m_state->screen, &background);
            QColor fg = VTERM_COLOR_IS_DEFAULT_FG(&cell.fg) ? defaultForeground
                : QColor(foreground.rgb.red, foreground.rgb.green, foreground.rgb.blue);
            QColor bg = VTERM_COLOR_IS_DEFAULT_BG(&cell.bg) ? defaultBackground
                : QColor(background.rgb.red, background.rgb.green, background.rgb.blue);
            if (cell.attrs.reverse) std::swap(fg, bg);
            const QRectF cellRect(column * m_state->cellWidth, row * m_state->cellHeight,
                m_state->cellWidth, m_state->cellHeight);
            painter.fillRect(cellRect, bg);
            if (cell.width == 0) continue;
            QFont cellFont = font();
            cellFont.setBold(cell.attrs.bold);
            cellFont.setItalic(cell.attrs.italic);
            cellFont.setUnderline(cell.attrs.underline != VTERM_UNDERLINE_OFF);
            cellFont.setStrikeOut(cell.attrs.strike);
            painter.setFont(cellFont);
            char32_t codepoints[VTERM_MAX_CHARS_PER_CELL]{};
            int count = 0;
            while (count < VTERM_MAX_CHARS_PER_CELL && cell.chars[count] != 0) {
                codepoints[count] = static_cast<char32_t>(cell.chars[count]); ++count;
            }
            if (count > 0 && !cell.attrs.conceal) {
                painter.setPen(fg);
                const QString glyph = QString::fromUcs4(codepoints, count);
                painter.drawText(QPointF(column * m_state->cellWidth,
                    row * m_state->cellHeight + m_state->ascent), glyph);
            }
        }
    }
    if (m_state->cursorVisible && hasFocus()) {
        painter.fillRect(QRectF(m_state->cursor.col * m_state->cellWidth,
            m_state->cursor.row * m_state->cellHeight, m_state->cellWidth, m_state->cellHeight),
            QColor(210, 220, 235, 110));
    }
}

void VtTerminalWidget::resizeEvent(QResizeEvent *event)
{
    QWidget::resizeEvent(event);
    const int columns = std::max(2, static_cast<int>(width() / m_state->cellWidth));
    const int rows = std::max(1, static_cast<int>(height() / m_state->cellHeight));
    if (columns == m_state->columns && rows == m_state->rows) return;
    m_state->columns = columns; m_state->rows = rows;
    vterm_set_size(m_state->terminal, rows, columns);
    emit terminalResized(columns, rows);
}

void VtTerminalWidget::keyPressEvent(QKeyEvent *event)
{
    const bool pasteShortcut = event->key() == Qt::Key_V
        && (event->modifiers().testFlag(Qt::MetaModifier)
            || (event->modifiers().testFlag(Qt::ControlModifier)
                && event->modifiers().testFlag(Qt::ShiftModifier)));
    if (pasteShortcut) {
        const QString text = QApplication::clipboard()->text();
        vterm_keyboard_start_paste(m_state->terminal);
        for (uint codepoint : text.toUcs4()) vterm_keyboard_unichar(m_state->terminal, codepoint, VTERM_MOD_NONE);
        vterm_keyboard_end_paste(m_state->terminal);
        flushInput();
        event->accept();
        return;
    }
    const VTermModifier vtModifiers = modifiers(event->modifiers());
    VTermKey key = VTERM_KEY_NONE;
    switch (event->key()) {
    case Qt::Key_Return: case Qt::Key_Enter: key = VTERM_KEY_ENTER; break;
    case Qt::Key_Tab: key = VTERM_KEY_TAB; break;
    case Qt::Key_Backspace: key = VTERM_KEY_BACKSPACE; break;
    case Qt::Key_Escape: key = VTERM_KEY_ESCAPE; break;
    case Qt::Key_Up: key = VTERM_KEY_UP; break;
    case Qt::Key_Down: key = VTERM_KEY_DOWN; break;
    case Qt::Key_Left: key = VTERM_KEY_LEFT; break;
    case Qt::Key_Right: key = VTERM_KEY_RIGHT; break;
    case Qt::Key_Home: key = VTERM_KEY_HOME; break;
    case Qt::Key_End: key = VTERM_KEY_END; break;
    case Qt::Key_PageUp: key = VTERM_KEY_PAGEUP; break;
    case Qt::Key_PageDown: key = VTERM_KEY_PAGEDOWN; break;
    case Qt::Key_Delete: key = VTERM_KEY_DEL; break;
    case Qt::Key_F1: key = static_cast<VTermKey>(VTERM_KEY_FUNCTION(1)); break;
    case Qt::Key_F2: key = static_cast<VTermKey>(VTERM_KEY_FUNCTION(2)); break;
    case Qt::Key_F3: key = static_cast<VTermKey>(VTERM_KEY_FUNCTION(3)); break;
    case Qt::Key_F4: key = static_cast<VTermKey>(VTERM_KEY_FUNCTION(4)); break;
    case Qt::Key_F5: key = static_cast<VTermKey>(VTERM_KEY_FUNCTION(5)); break;
    case Qt::Key_F6: key = static_cast<VTermKey>(VTERM_KEY_FUNCTION(6)); break;
    case Qt::Key_F7: key = static_cast<VTermKey>(VTERM_KEY_FUNCTION(7)); break;
    case Qt::Key_F8: key = static_cast<VTermKey>(VTERM_KEY_FUNCTION(8)); break;
    case Qt::Key_F9: key = static_cast<VTermKey>(VTERM_KEY_FUNCTION(9)); break;
    case Qt::Key_F10: key = static_cast<VTermKey>(VTERM_KEY_FUNCTION(10)); break;
    case Qt::Key_F11: key = static_cast<VTermKey>(VTERM_KEY_FUNCTION(11)); break;
    case Qt::Key_F12: key = static_cast<VTermKey>(VTERM_KEY_FUNCTION(12)); break;
    default: break;
    }
    if (key != VTERM_KEY_NONE) vterm_keyboard_key(m_state->terminal, key, vtModifiers);
    else if (!event->text().isEmpty()) {
        const auto characters = event->text().toUcs4();
        for (uint codepoint : characters) vterm_keyboard_unichar(m_state->terminal, codepoint, vtModifiers);
    } else { QWidget::keyPressEvent(event); return; }
    flushInput();
    event->accept();
}

VTermModifier VtTerminalWidget::modifiers(Qt::KeyboardModifiers modifiers) const
{
    VTermModifier result = VTERM_MOD_NONE;
    if (modifiers.testFlag(Qt::ShiftModifier)) result = static_cast<VTermModifier>(result | VTERM_MOD_SHIFT);
    if (modifiers.testFlag(Qt::AltModifier)) result = static_cast<VTermModifier>(result | VTERM_MOD_ALT);
    if (modifiers.testFlag(Qt::ControlModifier)) result = static_cast<VTermModifier>(result | VTERM_MOD_CTRL);
    return result;
}

void VtTerminalWidget::sendMouseButton(int button, bool pressed, const QPointF &position,
    Qt::KeyboardModifiers keyboardModifiers)
{
    const int row = std::clamp(static_cast<int>(position.y() / m_state->cellHeight), 0, m_state->rows - 1);
    const int column = std::clamp(static_cast<int>(position.x() / m_state->cellWidth), 0, m_state->columns - 1);
    vterm_mouse_move(m_state->terminal, row, column, modifiers(keyboardModifiers));
    vterm_mouse_button(m_state->terminal, button, pressed, modifiers(keyboardModifiers));
    flushInput();
}

void VtTerminalWidget::mousePressEvent(QMouseEvent *event)
{
    int button = 0;
    if (event->button() == Qt::LeftButton) button = 1;
    else if (event->button() == Qt::MiddleButton) button = 2;
    else if (event->button() == Qt::RightButton) button = 3;
    if (button == 0) { QWidget::mousePressEvent(event); return; }
    setFocus(Qt::MouseFocusReason);
    sendMouseButton(button, true, event->position(), event->modifiers());
    event->accept();
}

void VtTerminalWidget::mouseReleaseEvent(QMouseEvent *event)
{
    int button = 0;
    if (event->button() == Qt::LeftButton) button = 1;
    else if (event->button() == Qt::MiddleButton) button = 2;
    else if (event->button() == Qt::RightButton) button = 3;
    if (button == 0) { QWidget::mouseReleaseEvent(event); return; }
    sendMouseButton(button, false, event->position(), event->modifiers());
    event->accept();
}

void VtTerminalWidget::mouseMoveEvent(QMouseEvent *event)
{
    const int row = std::clamp(static_cast<int>(event->position().y() / m_state->cellHeight), 0, m_state->rows - 1);
    const int column = std::clamp(static_cast<int>(event->position().x() / m_state->cellWidth), 0, m_state->columns - 1);
    vterm_mouse_move(m_state->terminal, row, column, modifiers(event->modifiers()));
    flushInput();
    event->accept();
}

void VtTerminalWidget::wheelEvent(QWheelEvent *event)
{
    const int delta = event->angleDelta().y();
    if (delta == 0) { QWidget::wheelEvent(event); return; }
    const int button = delta > 0 ? 4 : 5;
    const int count = std::max(1, std::abs(delta) / 120);
    for (int index = 0; index < count; ++index) {
        sendMouseButton(button, true, event->position(), event->modifiers());
        sendMouseButton(button, false, event->position(), event->modifiers());
    }
    event->accept();
}
