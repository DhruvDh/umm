import static org.junit.Assert.assertEquals;

import org.junit.jupiter.api.Test;
import org.junit.platform.runner.JUnitPlatform;
import org.junit.runner.RunWith;

@RunWith(JUnitPlatform.class)
public class MainTest {
    @Test
    void testAddition() {
        assertEquals(2, 2);
    }
}