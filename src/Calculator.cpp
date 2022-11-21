#include "Calculator.hpp"
#include "Arduino.h"
#include "HID-Project.h"
#include <math.h>

Calculator::Calculator() {
    this->clearInput();
    this->clearResult();
}

unsigned char Calculator::getChar(unsigned char row, unsigned char column) const {
    return this->mapping[row][column];
}

void Calculator::onPress(const char row, const char column) {
    this->onPress(this->getChar(row, column));
}

void Calculator::onPress(const char input) {
    this->drawNext = true;

    if (input >= '0' && input <= '9') {
        this->doNumeric(input);
    } else {
        this->doOperation(input);
    }
}

void Calculator::loadMemory(const unsigned char slot) const {
    strcpy((char*)this->input, this->memory[slot]);
}

void Calculator::storeMemory(const unsigned char slot, const char* data) {
    strcpy(this->memory[slot], data);
}

void Calculator::onLongPress(const char row, const char column) {
    this->onLongPress(this->getChar(row, column));
}
void Calculator::onLongPress(const char input) {
    switch(input) {
        case 'a':
        case 'b':
        case 'c':
        case 'd':
            this->storeMemory(input - 'a', this->getResult());
            break;
        case '.':
        case 0:
            Keyboard.print(this->getResult());
            break;
    }
}

void Calculator::doNumeric(const char input) {
    if (this->staleInput) {
        this->clearInput();
    } else if (input == '0' && (this->input[0] == '0' || (this->input[0] == '-' && this->input[1] == '0'))) {
        return;
    }

    this->pushInput(input);
}

bool Calculator::pushInput(const char value) {
    if (this->staleInput) {
        this->input[0] = 0;
    }

    this->staleInput = false;
    return this->push(this->input, value);
}

bool Calculator::pushResult(const char value) {
    return this->push(this->result, value);
}

bool Calculator::push(char* target, const char value, const unsigned char size) {
    for(unsigned char i = 0; i < size; i++) {
        if (target[i] == 0) {
            target[i] = value;
            target[i+1] = 0;
            return true;
        }
    }

    return false;
}


void Calculator::doOperation(const char op) {
    // todo move if statements
    switch(op) {
        case 'a':
        case 'b':
        case 'c':
        case 'd':
            this->loadMemory(op - 'a');
            break;
        case '-':
            if (this->input[0] == 0 || this->staleInput) {
                strcpy(this->input, "-");
                this->staleInput = false;
                break;
            }
        case '+':
        case '/':
        case '*':
        case 'x':
            if (this->input[0] != 0 && !this->staleInput) {
                this->doMath(this->pendingOperation);
            }
            this->pendingOperation = op;
            this->staleInput = true;
            break;
        case 'C':
            if (this->input[0] == 0) {
                this->clearResult();
                this->pendingOperation = 0;
            } else {
                this->clearInput();
            }
            break;
        case '.':
            if (this->staleInput) {
                this->pushInput('0');
            }
            if (!this->hasPoint()) {
                this->pushInput('.');
            }
            break;
        case '\n':
            this->doMath(this->pendingOperation);
            this->staleInput = true;
            break;
    }
}

bool Calculator::hasPoint() const {
    for(unsigned char i = 0; i < CALC_VALUE_SIZE && this->input[i] > 0; i++) {
        if (this->input[i] == '.') {
            return true;
        }
    }

    return false;
}

const char* Calculator::getInput() const {
    return this->input[0] == 0 ? "0" : this->input;
}

const char* Calculator::getResult() const {
    return this->result[0] == 0 ? "0" : this->result;
}

void Calculator::doMath(const char op) {
    double input = atof(this->getInput());
    double result = atof(this->getResult());

    this->error = false;

    switch(op) {
        case '+':
            result += input;
            break;
        case '-':
            result -= input;
            break;
        case '*':
        case 'x':
            result *= input;
            break;
        case '/':
            if (input == 0) {
                this->error = true;
                return;
            }

            result /= input;
            break;
        default:
            strcpy(this->result, this->getInput());
            return;
    }

    String(result, CALC_PRECISION).toCharArray(this->result, CALC_VALUE_SIZE + 1);

    // trim zeros
    char* back = this->result + strlen(this->result);
    while((*--back) == '0');
    if((*back) == '.') back--;
    *(back+1) = '\0';
}

void Calculator::draw(U8G2* u8g2) {
    if (!this->drawNext) return;

    this->drawNext = false;

    u8g2->clearBuffer();
    
    u8g2->setFont(u8g2_font_ncenB08_tr);	// choose a suitable font
    
    const char operationStr[] = {this->pendingOperation, 0};

    u8g2->drawStr(0, 10, operationStr);
    u8g2->drawStr(10, 10, this->getInput());
    u8g2->drawStr(126 - u8g2->getStrWidth(this->getResult()), 31, this->getResult());

    if (this->error) {
        u8g2->drawStr(0, 31, "Err");
    }

    u8g2->sendBuffer();
}

void Calculator::onShow() {
    this->drawNext = true;

    this->clearInput();
    this->clearResult();
}

void Calculator::clearInput() {
    this->input[0] = 0;
}

void Calculator::clearResult() {
    this->result[0] = 0;
}