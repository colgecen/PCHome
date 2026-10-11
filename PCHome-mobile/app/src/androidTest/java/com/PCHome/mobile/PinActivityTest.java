package com.PCHome.mobile;

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

@RunWith(AndroidJUnit4.class)
@LargeTest
public class PinActivityTest {

    @Rule
    public ActivityScenarioRule<PinActivity> activityRule =
            new ActivityScenarioRule<>(PinActivity.class);

    @Test
    public void activityLaunches_displaysIdAndUpdate() {
        onView(withId(R.id.pc_id_input)).check(matches(isDisplayed()));
        onView(withId(R.id.password_input)).check(matches(isDisplayed()));
        onView(withId(R.id.update_section)).check(matches(isDisplayed()));
        onView(withId(R.id.update_open_button)).check(matches(isDisplayed()));
    }

    @Test
    public void connectButton_isDisplayed() {
        onView(withId(R.id.connect_button)).check(matches(isDisplayed()));
        onView(withId(R.id.connect_button)).check(matches(withText(R.string.connect)));
    }

    @Test
    public void recentGrid_isDisplayed() {
        onView(withId(R.id.recent_grid)).check(matches(isDisplayed()));
    }

    @Test
    public void idField_acceptsInput() {
        onView(withId(R.id.pc_id_input)).perform(typeText("482913"), closeSoftKeyboard());
        onView(withId(R.id.pc_id_input)).check(matches(withText("482913")));
    }

    @Test
    public void updateDialog_opensFromSection() {
        onView(withId(R.id.update_open_button)).perform(click());
        onView(withText(R.string.update_check_now)).check(matches(isDisplayed()));
    }

    @Test
    public void connectWithShortId_doesNotCrash() {
        onView(withId(R.id.pc_id_input)).perform(typeText("12"), closeSoftKeyboard());
        onView(withId(R.id.connect_button)).perform(click());
        onView(withId(R.id.connect_button)).check(matches(isDisplayed()));
    }
}
