package com.pchome.mobile;

import androidx.test.ext.junit.rules.ActivityScenarioRule;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.filters.LargeTest;

import org.junit.Rule;
import org.junit.Test;
import org.junit.runner.RunWith;

import static androidx.test.espresso.Espresso.onView;
import static androidx.test.espresso.assertion.ViewAssertions.matches;
import static androidx.test.espresso.matcher.ViewMatchers.isDisplayed;
import static androidx.test.espresso.matcher.ViewMatchers.withId;

@RunWith(AndroidJUnit4.class)
@LargeTest
public class TouchpadActivityTest {

    @Rule
    public ActivityScenarioRule<TouchpadActivity> activityRule =
            new ActivityScenarioRule<>(TouchpadActivity.class);

    @Test
    public void activityLaunches_displaysTouchpad() {
        onView(withId(R.id.touchpad_container)).check(matches(isDisplayed()));
    }

    @Test
    public void hotkeysAreDisplayed() {
        onView(withId(R.id.btn_left_click)).check(matches(isDisplayed()));
        onView(withId(R.id.btn_right_click)).check(matches(isDisplayed()));
    }
}
