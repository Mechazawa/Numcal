#include "Numpad.hpp"

unsigned char Numpad::getChar(unsigned char row, unsigned char column) const {
    return this->mapping[1][row][column];
    //return this->mapping[this->numlock][row][column];
}

void Numpad::onPress(char row, char column) {
    char _char = this->getChar(row, column);

    Keyboard.release(_char);

    if (_char == KEY_NUM_LOCK) {
        this->numlock = !this->numlock;
    }
}

void Numpad::onRelease(char row, char column) {
    Keyboard.release(this->getChar(row, column));
}