#!/usr/bin/bash
export CLASSPATH="./junit.jar:./hamcrest.jar:./bin"

javac -d bin -Xlint:unchecked MainTest.java

java org.junit.runner.JUnitCore MainTest

rm -rf bin/*