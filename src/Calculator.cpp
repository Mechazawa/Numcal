#include "Calculator.hpp"


// ECalculatorMode Calculator::getMode() const {
//     return this->mode;
// }

// void Calculator::setMode(ECalculatorMode mode) {
//     if (mode >= CYCLE_BACK) {
//         mode = ECalculatorMode::SIMPLE;
//     }

//     this->mode = mode;
// }

// void Calculator::nextMode() {
//     this->setMode((ECalculatorMode)(this->getMode() + 1));
// }

unsigned char Calculator::getChar(unsigned char row, unsigned char column) const {
    return this->mapping[row][column];
}

void Calculator::onPress(char row, char column) {
    char input = this->getChar(row, column);

    if (input < 10) {
        this->doNumeric(input);
    } else {
        this->doOperation(input);
    }
}

void Calculator::doNumeric(char input) {
    if (this->clearNext) {
        this->input = 0;
        this->inputOffset = 0;
        this->clearNext = false;
    }
    
    if (this->inputOffset > 0 || this->operation == '.') {
        this->inputOffset ++;
    }

    this->operation = 0;
    this->input = (this->input * 10) + input;
}

void Calculator::doOperation(char op) {
    // todo move if statements
    switch(op) {
        case '+':
        case '-':
        case '/':
        case '*':
            if (this->operation != op && !this->clearNext) {
                this->doMath(this->operation);
            }

            this->operation = op;
            break;
        case 'C':
            if (this->operation == 'C') {
                this->result = 0;
            } 

            this->input = 0;
            this->inputOffset = 0;
            // fallthru
        case '.':
            this->operation = op;
            break;
        case '\n':
            this->doMath(this->operation);
            break;
    }
}

double Calculator::getInput() const {
    return this->input / (10 ^ this->inputOffset);
}

double Calculator::getResult() const {
    return this->result;
}

void Calculator::doMath(char op) {
    switch(op) {
        case '+':
            this->result += this->input;
            break;
        case '-':
            this->result -= this->input;
            break;
        case '*':
            this->result *= this->input;
            break;
        case '/':
            this->result /= this->input;
            break;
    }

    this->clearNext = true;
}