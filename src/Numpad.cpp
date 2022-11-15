#include "Numpad.hpp"

KeyboardKeycode Numpad::getChar(unsigned char row, unsigned char column) const {
    return this->mapping[this->numlock][row][column];
}

void Numpad::onPress(char row, char column) {
    KeyboardKeycode _char = this->getChar(row, column);

    NKROKeyboard.press(_char);
    NKROKeyboard.send();

    if (_char == KEY_NUM_LOCK) {
        this->numlock = !this->numlock;
    }
}

void Numpad::onRelease(char row, char column) {
    NKROKeyboard.release(this->getChar(row, column));
    NKROKeyboard.send();
}