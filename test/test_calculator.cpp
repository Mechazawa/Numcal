#include <Arduino.h>
#include <SPI.h>
#include <Wire.h>
#include "HID-Project.h"
// #include "Calculator.hpp"
#include <unity.h>

// Calculator calc;

// set stuff up here
void setUp(void) {
    
}

// clean stuff up here
void tearDown(void) {

}

void test_calculator_numeric_input(void) {
    // calc.onPress(1);
    // calc.onPress(2);
    // calc.onPress(3);
    // calc.onPress(4);

    // TEST_ASSERT_EQUAL_CHAR_ARRAY("1234", calc.getInput(), 5);
}

void setup() {
    // NOTE!!! Wait for >2 secs
    // if board doesn't support software reset via Serial.DTR/RTS
    pinMode(LED_BUILTIN, OUTPUT);
    digitalWrite(LED_BUILTIN, HIGH);
    delay(2000);
    digitalWrite(LED_BUILTIN, LOW);
    UNITY_BEGIN();
    RUN_TEST(test_calculator_numeric_input);
    UNITY_END();
    digitalWrite(LED_BUILTIN, HIGH);
}

void loop() {

}
