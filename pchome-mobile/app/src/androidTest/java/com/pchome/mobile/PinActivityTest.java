package com.pchome.mobile;

import androidx.test.ext.junit.rules.ActivityScenarioRule;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.filters.LargeTest;

import org.junit.Rule;
import org.junit.Test;
import org.junit.runner.RunWith;

import static androidx.test.espresso.Espresso.onView;
import static androidx.test.espresso.action.ViewActions.click;
import static androidx.test.espresso.action.ViewActions.closeSoftKeyboard;
import static androidx.test.espresso.action.ViewActions.typeText;
import static androidx.test.espresso.assertion.ViewAssertions.matches;
import static androidx.test.espresso.matcher.ViewMatchers.isDisplayed;
import static androidx.test.espresso.matcher.ViewMatchers.withId;
import static androidx.test.espresso.matcher.ViewMatchers.withText;
import static org.hamcrest.Matchers.not;

@RunWith(AndroidJUnit4.class)
@LargeTest
public class PinActivityTest {

    @Rule
    public ActivityScenarioRule<PinActivity> activityRule =
            new ActivityScenarioRule<>(PinActivity.class);

    @Test
    public void activityLaunches_displaysPinAndStatus() {
        onView(withId(R.id.pin)).check(matches(isDisplayed()));
        onView(withId(R.id.status)).check(matches(isDisplayed()));
        onView(withId(R.id.status)).check(matches(withText(R.string.idle)));
    }

    @Test
    public void connectButton_isDisplayed() {
        onView(withId(R.id.connect_button)).check(matches(isDisplayed()));
        onView(withId(R.id.connect_button)).check(matches(withText(R.string.connect)));
    }

    @Test
    public void pinField_acceptsInput() {
        onView(withId(R.id.pin)).perform(typeText("123456"), closeSoftKeyboard());
        onView(withId(R.id.pin)).check(matches(withText("123456")));
    }

    @Test
    public void connectWithShortPin_doesNotCrash() {
        onView(withId(R.id.pin)).perform(typeText("12"), closeSoftKeyboard());
        onView(withId(R.id.connect_button)).perform(click());
        onView(withId(R.id.connect_button)).check(matches(isDisplayed()));
    }
}
